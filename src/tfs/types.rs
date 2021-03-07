// mod disk;
use super::disk::Disk;
// use super::utility;

pub const MAGIC_NUMBER: usize = 0xf0f03410;
pub const POINTERS_PER_INODE: usize = 5;
pub const INODE_SIZE: usize = 32; // the inode size in bytes
pub const INODES_PER_BLOCK: usize = Disk::BLOCK_SIZE / INODE_SIZE;
pub const POINTERS_PER_BLOCK: usize = 1024; // this is calculated as: disk.BLOCK_SIZE / 4

#[derive(Copy, Clone, Debug)]
#[allow(dead_code)]
pub struct Superblock {
    pub magic_number: u32,
    pub blocks: u32,
    pub inode_blocks: u32,  // there is no need for this field
    pub current_block_index: u32,
    pub free_lists: u32,
    pub inodes: u32
}

#[derive(Copy, Clone, Debug)]
#[allow(dead_code)]
pub struct Inode {
    pub valid: u32, // whether or not inode is valid (or allocated)
    pub size: u32,  // size of file
    pub ctime: u32,
    pub atime: u32,
    pub blk_pointer_level: u32, // level 1 = direct, level 2 = single_indirect, level 3 = double_indirect etc.
    pub level_index: u32,
    pub data_block: u32,    // pointer to the data blocks
    pub total_data_blocks: u32   // total number of data blocks allocated to this inode
}

#[derive(Copy, Clone, Debug)]
pub struct InodeBlock {
    pub inodes: [Inode; INODES_PER_BLOCK - 1],
    pub next_block: u32,
    pub prev_block: u32,
    pub temp: [u32; 6]  // 4 + 4 + 6 * 4 = 32 bytes => size of Inode
}

#[derive(Copy, Clone)]
#[allow(dead_code)]
pub union Block {
    pub superblock: Superblock,
    pub inode_block: InodeBlock,
    pub pointers: [u32; POINTERS_PER_BLOCK],
    pub data: [u8; Disk::BLOCK_SIZE]
}

#[derive(Copy, Clone)]
#[allow(dead_code)]
pub struct MetaData {
    pub superblock: Superblock,
    pub inodes_root_block: Block,
    pub root_free_list: Block,
    pub inodes_free_list: Block
}


#[allow(dead_code)]
impl Block {
    pub fn new() -> Self {
        Block {
            data: [0; Disk::BLOCK_SIZE]
        }
    }

    // ***************** get and set methods for Block fields *******************************
    // NOTE: the get methods ALWAYS returns a FRESH COPY of the field values because
    //      the fields are NOT reference types; they are OWNED types. Hence,
    //      if you still want to refer to the original field value, use the corresponding
    //      set methods. For example, 
    ///    let block = Block::new()
    ///    let mut data = block.data()   // 'data' would be a fresh copy of block.Data;
    ///    data[1] = 4;     // this won't change the corresponding block.Data
    //     however, if you wish to have this change reflect on Block.Data, use the set method:
    ///    block.set_data(data)
    // **************************************************************************************

    pub fn data(&self) -> [u8; Disk::BLOCK_SIZE] {
        unsafe {
            self.data
        }
    }

    pub fn data_as_mut(&mut self) -> &mut [u8; Disk::BLOCK_SIZE] {
        unsafe {
            &mut self.data
        }
    }

    pub fn data_as_ref(&self) -> &[u8; Disk::BLOCK_SIZE] {
        unsafe {
            &self.data
        }
    }

    pub fn superblock(&self) -> Superblock {
        unsafe {
            self.superblock
        }
    }

    pub fn inode_block(&self) -> InodeBlock {
        unsafe {
            self.inode_block
        }
    }

    pub fn iblock_as_mut(&mut self) -> &mut InodeBlock {
        unsafe {
            &mut self.inode_block
        }
    }

    pub fn iblock_as_ref(&self) -> &InodeBlock {
        unsafe {
            &self.inode_block
        }
    }

    pub fn pointers(&self) -> [u32; POINTERS_PER_BLOCK]{
        unsafe {
            self.pointers
        }
    }

    pub fn pointers_as_mut(&mut self) -> &mut [u32; POINTERS_PER_BLOCK]{
        unsafe {
            &mut self.pointers
        }
    }

    pub fn pointers_as_ref(&self) -> &[u32; POINTERS_PER_BLOCK]{
        unsafe {
            &self.pointers
        }
    }


    // setters
    pub fn set_data(&mut self, data: [u8; Disk::BLOCK_SIZE]) {
        self.data = data;
    }

    pub fn set_inode_block(&mut self, inode_block: InodeBlock) {
        self.inode_block = inode_block;
    }

    pub fn set_pointers(&mut self, pointers: [u32; POINTERS_PER_BLOCK]) {
        self.pointers = pointers;
    }

    pub fn set_superblock(&mut self, superblock: Superblock) {
        self.superblock = superblock;
    }
}

impl Inode {
    pub fn blank() -> Self {
        Inode {
            valid: 1,
            size: 0,
            atime: 0,
            ctime: 0,
            blk_pointer_level: 1,  // start from direct pointers
            level_index: 0,
            total_data_blocks: 0,
            data_block: 0
        }
    }

    pub fn increase_level(&mut self) -> bool {
        match self.level_index {
            1 => { 
                self.level_index = 2;
             }
            2 => { self.level_index = 3; }
            _ => {
                return false;
            }
        };
        return true;
    }
}

impl InodeBlock {
    pub fn new() -> Self {
        Block::new().inode_block()
    }

    pub fn as_block(self) -> Block {
        let mut block = Block::new();
        block.set_inode_block(self);
        block
    }

    pub fn from_inode(inode: Inode) -> Self {
        let mut inode_blk = Self::new();
        inode_blk.inodes[0] = inode;
        inode_blk
    }
}


// impl Iterator for InodeBlock {
//     type Item = [Inode; INODES_PER_BLOCK - 1];
//     fn next(&mut self) -> Option<Self::Item> {

//     }
// }

// InodeList iterator keeps track of the current inodeBlock and is responsible for
// adding new inodeBlock to the list when the block is full
// returning the next inode in the current inodeBlock.
// The inodeList is also contains an iterator for each (file) inode within the list

pub struct InodeList<'a> {
    curr_inode_block: InodeBlock,
    fs_meta_data: &'a mut MetaData,
    inode_index: usize,
    disk: Disk<'a>
}

impl<'a> InodeList<'a> {
    pub fn new(fs_meta_data: &'a mut MetaData, disk: Disk<'a>) -> Self {
        let inode_blk = fs_meta_data.inodes_root_block.inode_block();
        let k = INODES_PER_BLOCK - 1;
        let n = fs_meta_data.superblock.inodes as usize;
        let mut i = n % k;
        if n != 0 && i == 0 {
            i = k;
        }
        
        InodeList{
            disk,
            fs_meta_data,
            inode_index: i ,
            curr_inode_block: inode_blk
        }
    }

    pub fn add(&mut self, inode: Inode) -> i64 {
        if self.inodeblock_isfull() {
            -1
        } else {
            let inumber = self.fs_meta_data.superblock.inodes as usize;
            let mut inode = inode;
            inode.valid = inumber as u32;
            self.fs_meta_data.inodes_root_block
                .iblock_as_mut()
                .inodes[self.inode_index] = inode;
            self.inode_index += 1;
            self.fs_meta_data.superblock.inodes += 1;
            // println!("index: {}, INUMBER: {}, Total Inodes: {}", self.inode_index, inumber, self.fs_meta_data.superblock.inodes);
            inumber as i64
        }
    }

    pub fn inodeblock_isfull(&self) -> bool {
        if self.inode_index == INODES_PER_BLOCK - 1 {
            true
        } else {
            false
        }
    }

    pub fn add_inodeblock(&mut self, new_blk: i64) -> bool {
        if new_blk > 0 {
            // println!("Adding a new block: {}", new_blk);
            let mut inode_blk = InodeBlock::new();
            inode_blk.next_block = new_blk as u32;
            let block = inode_blk.as_block();

            let mut root_blk = self.fs_meta_data.inodes_root_block;
            root_blk.iblock_as_mut().prev_block = new_blk as u32;
            self.disk.write(new_blk as usize, root_blk.data_as_mut());
            self.fs_meta_data.inodes_root_block = block;
            self.inode_index = 0;
            true
        } else {
            false
        }
    }

    pub fn iter(&self) -> InodeListIter<'a> {
        InodeListIter::new(
            self.fs_meta_data.superblock.inodes as usize,
            self.curr_inode_block,
            self.disk.clone()
        )
    }
}

// ITERATOR FOR INODE LIST
pub struct InodeListIter<'b> {
    curr_inode_block: InodeBlock,
    next_i: usize,  // index used by the iterator
    total_items: usize,
    disk: Disk<'b>
}

impl<'b> InodeListIter<'b> {
    pub fn new(total_items: usize, start_inode_block: InodeBlock, disk: Disk<'b>) -> Self {
        InodeListIter {
            total_items,
            curr_inode_block: start_inode_block,
            next_i: 0,
            disk
        }
    }

    pub fn end_of_curr_blk(&self) -> bool {
        if self.next_i % INODES_PER_BLOCK == INODES_PER_BLOCK - 1 {
            true
        } else {
            false
        }
    }

    fn get_next(&mut self) -> Option<Inode> {
        // println!("INDEX:{}, {}", self.next_i, self.next_i % (INODES_PER_BLOCK));
        if self.next_i < self.total_items {
            let inode = self.curr_inode_block
            .inodes[self.next_i % (INODES_PER_BLOCK - 1)];
            self.next_i += 1;
            Some(inode)
        } else {
            None
        }
    }
}

impl<'b> Iterator for InodeListIter<'b> {
    type Item = Inode;

    fn next(&mut self) -> Option<Self::Item> {
        if self.end_of_curr_blk() {
            let next_iblock = self.curr_inode_block.next_block;
            if next_iblock > 0 {
                let mut block = Block::new();
                self.disk.read(next_iblock as usize, block.data_as_mut());
                self.curr_inode_block = block.inode_block();
            }
            return self.get_next()
        } else {
            return self.get_next()
        }
        Some(Inode::blank())
    }
}