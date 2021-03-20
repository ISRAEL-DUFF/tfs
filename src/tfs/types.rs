// mod disk;
use super::disk::Disk;
// use super::utility;

pub const MAGIC_NUMBER: usize = 0xf0f03410;
pub const POINTERS_PER_INODE: usize = 5;
pub const INODE_SIZE: usize = 32; // the inode size in bytes
pub const INODES_PER_BLOCK: usize = Disk::BLOCK_SIZE / INODE_SIZE;
pub const POINTERS_PER_BLOCK: usize = 1024; // this is calculated as: disk.BLOCK_SIZE / 4

pub const INODES_ROOT_BLOCK: usize = 1;
pub const ROOT_FREE_LIST: usize = 2;
pub const INODES_FREE_LIST: usize = 3;

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

pub struct InodeProxy<'c> {
    inumber: usize,
    inode_table: &'c mut Vec<(u32, InodeBlock)>,
    fs_meta_data: &'c mut MetaData,
    disk: Disk<'c>
}

pub struct InodeDataPointer<'d> {
    pointers: Vec<u32>,
    disk: &'d mut Disk<'d>,
    root_ptr: u32
}

#[derive(Copy, Clone, Debug)]
pub struct InodeBlock {
    pub inodes: [Inode; INODES_PER_BLOCK - 1],
    pub next_block: u32,  // 4 bytes +
    pub prev_block: u32,  // 4 bytes +
    pub temp: [u32; 6]   //  6 * 4 = 32 bytes => size of 1 Inode
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


// ************ UTILITY FUNCTIONS ************************************
pub fn fetch_block(disk: &mut Disk, block_num: usize) -> Block {
    let mut block = Block::new();
    disk.read(block_num, block.data_as_mut());
    return block;
}

pub fn get_inode_block(inode_table: &Vec<(u32, InodeBlock)>, i: usize) -> (u32, Block) {
    let index = (i as f64 / (INODES_PER_BLOCK - 1) as f64).floor() as usize;
    let (block_num, inodeblock) = inode_table[index];
    // println!("GET INODE BLOCK: {}, {}", index, block_num);
    (block_num, inodeblock.as_block())
}

pub fn set_inode(inode_table: &mut Vec<(u32, InodeBlock)>, inumber: usize, inode: Inode) {
    let (blk_n, mut block) = get_inode_block(inode_table, inumber);
    let i = inumber % (INODES_PER_BLOCK - 1);
    block.iblock_as_mut().inodes[i] = inode;
    // println!("SET INODE ==> Inumber: {}, inode: {:?}", inumber, inode);
    let index = (inumber as f64 / (INODES_PER_BLOCK - 1) as f64).floor() as usize;
    inode_table[index] = (blk_n, block.inode_block());
}

// ************ UTILITY FUNCTIONS ************************************

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

    pub fn pointer_level(&self) -> u32 {
        self.blk_pointer_level
    }
}

impl<'d> InodeDataPointer<'d> {
    pub fn new<'e:'d>(disk: &'e mut Disk<'d>, root_ptr: u32) -> Self {
        InodeDataPointer {
            root_ptr,
            disk,
            pointers: Vec::new()
        }
    }

    pub fn level(mut self, n: usize) -> Self {
        let data_block = self.root_ptr;
        let index_zero = |array: [u32; POINTERS_PER_BLOCK]| {
            let mut i = 0;
            loop {
                if i == array.len() || array[i] == 0 {
                    break
                }
                i += 1;
            }
            i
        };
        match n {
            1 => {  // single indirect pointers
                let mut block = Block::new();
                self.disk.read(self.root_ptr as usize, block.data_as_mut());
                let i = index_zero(block.pointers());
                self.pointers = Vec::from(&block.pointers()[0..i]);
            },
            _ => {
                let mut data_blks = Block::new();
                let mut data_blocks = Vec::new();
                for blk in self.pointers.iter() {
                    if *blk == 0 {
                        break
                    }
                    self.disk.read(*blk as usize, data_blks.data_as_mut());
                    let i = index_zero(data_blks.pointers());
                    data_blocks.extend_from_slice(&data_blks.pointers()[..i]);
                }
                self.pointers = data_blocks;
            }
        }
        //
        self
    }

    pub fn blocks(self) -> Vec<u32> {
        self.pointers
    }
}

impl<'c> InodeProxy<'c> {
    pub fn new(fs_meta_data: &'c mut MetaData,
        inode_table: &'c mut Vec<(u32, InodeBlock)>, 
        disk: Disk<'c>, 
        inumber: usize
    ) -> Self {
        InodeProxy {
            inumber,
            inode_table,
            fs_meta_data,
            disk
        }
    }

    fn get_index(&self) -> (usize, usize) {
        let inode_block_index = (self.inumber as f64 / (INODES_PER_BLOCK - 1) as f64).floor() as usize;
        let inode_index = self.inumber % (INODES_PER_BLOCK - 1);
        (inode_block_index, inode_index)
    }

    pub fn size(&self) -> u32 {
        let (i, j) = self.get_index();
        self.inode_table[i].1.inodes[j].size
    }

    pub fn data_block(&self) -> u32 {
        let (i, j) = self.get_index();
        self.inode_table[i].1.inodes[j].data_block
    }
    pub fn pointer_level(&self) -> u32 {
        let (i, j) = self.get_index();
        self.inode_table[i].1.inodes[j].blk_pointer_level
    }

    pub fn set_data_block(&mut self, blk: u32) {
        let (i, j) = self.get_index();
        self.inode_table[i].1.inodes[j].data_block = blk;
    }

    pub fn incr_size(&mut self, amount: usize) {
        let (i, j) = self.get_index();
        self.inode_table[i].1.inodes[j].size += amount as u32;
    }

    pub fn incr_data_blocks(&mut self, amount: usize) {
        let (i, j) = self.get_index();
        self.inode_table[i].1.inodes[j].total_data_blocks += amount as u32;
        let data_blocks = self.inode_table[i].1.inodes[j].total_data_blocks;

        if data_blocks as usize == POINTERS_PER_BLOCK || 
          data_blocks as usize == POINTERS_PER_BLOCK.pow(2) || 
          data_blocks as usize == POINTERS_PER_BLOCK.pow(3) {
            self.inode_table[i].1.inodes[j].blk_pointer_level += 1;
            // println!("*******Increased pointer level******")
        }
    }

    pub fn save(&mut self) -> bool {
        let (i, j) = self.get_index();
        let (block_num, mut inodeblock) = self.inode_table[i];
        self.disk.write(block_num as usize, inodeblock.as_block().data_as_mut());

        if block_num as usize == INODES_ROOT_BLOCK {
            self.fs_meta_data.inodes_root_block.iblock_as_mut().inodes = inodeblock.inodes;
        }
        true
    }


    pub fn save_data_blocks(&mut self, data_blocks: &Vec<u32>) {
        // println!("Pointer Data Block: {}, level: {}, {}", self.data_block(), self.pointer_level(), data_blocks.len());
        let root_block = fetch_block(&mut self.disk.clone(), self.data_block() as usize);
        match self.pointer_level() {
            1 => {
                // println!("1st level");
                let mut i = 0;
                let mut direct_pointers = Block::new();
                for j in 0..root_block.pointers().len() {
                    if i < data_blocks.len() && data_blocks[i] > 0 {
                        direct_pointers.pointers_as_mut()[j as usize] = data_blocks[i];
                        i += 1;
                    } else {
                        break
                    }
                }
                if self.data_block() > 0 {
                    self.disk.write(self.data_block() as usize, direct_pointers.data_as_mut());
                }
            },
    
            2 => {
                // println!("2nd level");
                let mut i = 0;
                for ptr1 in root_block.pointers().iter() {
                    if *ptr1 > 0 {
                        let mut direct_pointers = Block::new();
                        for j in 0..POINTERS_PER_BLOCK {
                            if i < data_blocks.len() {
                                direct_pointers.pointers_as_mut()[j] = data_blocks[i];
                                i += 1;
                            } else {
                                break
                            }
                        }
                        self.disk.write(*ptr1 as usize, direct_pointers.data_as_mut());
                    } else {
                        break
                    }
                }
            },
    
            3 => {
                let mut i = 0;
                for ptr1 in root_block.pointers().iter() {
                    if *ptr1 > 0 {
                        let root_2_blocks = fetch_block(&mut self.disk, *ptr1 as usize);
                        for ptr2 in root_2_blocks.pointers().iter() {
                            if *ptr2 > 0 {
                                let mut direct_pointers = Block::new();
                                for j in 0..POINTERS_PER_BLOCK {
                                    if i < data_blocks.len() {
                                        direct_pointers.pointers_as_mut()[j] = data_blocks[i];
                                        i += 1;
                                    } else {
                                        break
                                    }
                                }
                                self.disk.write(*ptr2 as usize, direct_pointers.data_as_mut());
                            } else {
                                break
                            }
                        }
                    } else {
                        break
                    }
                }
            },
    
            _ => {}
        }
    }

    fn to_direct_pointers(&mut self) -> Vec<u32> {
        let data_block = self.data_block();
        let index_zero = |array: [u32; POINTERS_PER_BLOCK]| {
            let mut i = 0;
            loop {
                if i == array.len() || array[i] == 0 {
                    break
                }
                i += 1;
            }
            i
        };
        match self.pointer_level() {
            1 => {  // single indirect pointers
                if data_block == 0 {
                    return vec![]
                }
                let mut block = Block::new();
                self.disk.read(self.data_block() as usize, block.data_as_mut());
                let i = index_zero(block.pointers());
                Vec::from(&block.pointers()[0..i])
            },
            2 => {
                if data_block == 0 {
                    return vec![]
                }
                let mut block = Block::new();
                self.disk.read(self.data_block() as usize, block.data_as_mut());

                let mut data_blks = Block::new();
                let mut data_blocks = Vec::new();
                for blk in block.pointers().iter() {
                    if *blk == 0 {
                        break
                    }
                    self.disk.read(*blk as usize, data_blks.data_as_mut());
                    let i = index_zero(data_blks.pointers());
                    data_blocks.extend_from_slice(&data_blks.pointers()[..i]);
                }
                data_blocks
            },
            3 => {
                if data_block == 0 {
                    return vec![]
                }
                let mut block = Block::new();
                self.disk.read(self.data_block() as usize, block.data_as_mut());

                let mut data_blks_1 = Block::new();
                let mut data_blks_2 = Block::new();
                let mut data_blocks = Vec::new();
                for blk in block.pointers().iter() {
                    if *blk == 0 {
                        break
                    }
                    self.disk.read(*blk as usize, data_blks_1.data_as_mut());
                    for b in data_blks_1.pointers().iter() {
                        if *b == 0 {
                            break
                        }
                        self.disk.read(*b as usize, data_blks_2.data_as_mut());
                        let i = index_zero(data_blks_2.pointers());
                        data_blocks.extend_from_slice(&data_blks_2.pointers()[..i]);
                    }
                }
                data_blocks
            },
            _ => vec![]
        }
    }
}


pub struct InodeWriteIter<'a> {
    inumber: usize,
    disk: Disk<'a>,
    buffer: [u8; Disk::BLOCK_SIZE],
    data_blocks: Vec<u32>,
    curr_data_block: Block,
    data_block_num: u32,
    data_block_index: usize,
    next_write_index: usize,
    flushed: bool,
    aligned_to_block: bool,
    inode_table: &'a mut Vec<(u32, InodeBlock)>,
    fs_meta_data: &'a mut MetaData
}

impl<'a> InodeWriteIter<'a> {
    pub fn new<'b: 'a>(
        inumber: usize, mut disk: Disk<'b>, fs_meta_data: &'b mut MetaData, 
        inode_table: &'b mut Vec<(u32, InodeBlock)>
    ) -> Self {
        let i_table = inode_table as *mut Vec<(u32, InodeBlock)>;
        let meta_data = fs_meta_data as *mut MetaData;
        unsafe {
            let mut inode = InodeProxy::new(&mut (*meta_data), &mut (*i_table), disk.clone(), inumber);
            let disc = disk.clone();
            let data_blocks = inode.to_direct_pointers();
            
            let mut i_write = InodeWriteIter {
                inumber,
                disk,
                buffer: [0; Disk::BLOCK_SIZE],
                data_blocks,
                curr_data_block: Block::new(),
                data_block_num: 0,
                data_block_index: 0,
                next_write_index: 0,
                flushed: false,
                aligned_to_block: false,
                inode_table: &mut (*i_table),
                fs_meta_data: &mut (*meta_data)
            };
            i_write.seek(inode.size() as usize);
            i_write
        }
    }

    pub fn seek(&mut self, offset: usize) {
        let inode = self.get_inode();
        if offset as u32 > inode.size() {
            let offset = inode.size();
        }

        self.data_block_index = (offset as f64 / Disk::BLOCK_SIZE as f64).floor() as usize;
        self.next_write_index = offset % Disk::BLOCK_SIZE;
        if self.data_block_index < self.data_blocks.len() {
            self.data_block_num = self.data_blocks[self.data_block_index];
            let mut curr_blk = Block::new();
            self.disk.read(self.data_block_num as usize, curr_blk.data_as_mut());
            self.curr_data_block = curr_blk;
        } else {
            self.next_write_index = Disk::BLOCK_SIZE;
            self.data_block_num = 0;
            self.curr_data_block = Block::new();
        }
    }

    pub fn has_datablock(&mut self) -> bool {
        if self.get_inode().data_block() > 0 {
            true
        } else {
            false
        }
    }

    pub fn set_data_block(&mut self, blk_num: i64) -> bool {
        if blk_num > 0 {
            let mut inode = self.get_inode();
            inode.set_data_block(blk_num as u32);
            self.save_inode();
            true
        } else {
            false
        }
    }

    pub fn write_byte(&mut self, byte: u8) -> (bool, i64) {
        if self.next_write_index < Disk::BLOCK_SIZE && self.data_blocks.len() > 0 {
            self.curr_data_block.data_as_mut()[self.next_write_index] = byte;
            self.next_write_index += 1;
            self.aligned_to_block = false;
            self.flushed = false;
            (true, 1)
        } else {
            // flush current block
            if !self.flush() {
                println!("No flush {}", self.data_blocks.len());
                return (false, -1);
            };

            // add another block as current block
            if self.data_block_index < self.data_blocks.len() - 1 {
                self.data_block_index += 1;
                self.data_block_num = self.data_blocks[self.data_block_index];
                let mut data_block = Block::new();
                self.disk.read(self.data_block_num.clone() as usize, data_block.data_as_mut());
                self.curr_data_block = data_block;

                self.curr_data_block.data_as_mut()[self.next_write_index] = byte;
                self.next_write_index += 1;
                self.aligned_to_block = false;
                self.flushed = false;
                return (true, 1)
            } else {    // add new data block
                (false, -1)
            }
        }
    }

    // the align_to_block() method is used to align the write position to the beginning of a new block
    // this enables the use of write_block() method.
    pub fn align_to_block(&mut self, block_data: &mut [u8]) -> (bool, i64) {
        if self.aligned_to_block {
            return (true, 0)
        }

        let n = Disk::BLOCK_SIZE - self.next_write_index;
        if block_data.len() < n {
            return (false, -1)
        }

        if self.get_inode().size() == 0 || self.next_write_index == 0 || self.next_write_index == Disk::BLOCK_SIZE {
            self.aligned_to_block = true;
            return (true, 0)
        }

        let mut m = 0;
        loop {
            let (success, _) = self.write_byte(block_data[m]);
            if !success {
                break
            }
            m += 1;
        }
        self.aligned_to_block = true;
        (true, m as i64)
    }

    pub fn write_block(&mut self, block_num: i64, block_data: &mut [u8]) -> bool {
        if block_num < 0  || !self.aligned_to_block {
            return false
        }
        if block_data.len() < Disk::BLOCK_SIZE {
            return false
        }

        if !self.add_data_blk(block_num) {
            return false
        }
        self.disk.write(block_num as usize, block_data);
        self.aligned_to_block = true;

        let mut inode = self.get_inode();
        inode.incr_size(Disk::BLOCK_SIZE);
        // inode.save();
        true
    }

    pub fn flush(&mut self) -> bool {
        // write current block to disk
        // increase file size and reset next write index
        let this = self as *mut Self;
        if self.data_block_num > 0 && self.data_blocks.len() > 0 {
            unsafe {
                if !self.aligned_to_block && !self.flushed {
                    (*this).disk.write(self.data_block_num as usize, self.curr_data_block.data_as_mut());
                    (*this).get_inode().incr_size(self.next_write_index);
                    println!("Wrote to block: {}", self.data_block_num);
                 } 
                
                // println!("F: {}, {}, {}", self.data_block_num, self.data_blocks.len(), self.next_write_index);
                if (*this).next_write_index == Disk::BLOCK_SIZE {
                    (*this).next_write_index = 0;
                }
                (*this).flushed = true;
                (*this).save_inode();
            }
            true
        } else {
            // println!("DId not flush: {}, {}", self.data_block_num, self.data_blocks.len());
            false
        }
    }

    pub fn add_data_blk(&mut self, new_blk: i64) -> bool {
        println!("Adding block: {}", new_blk);
        if new_blk > 0 {
            let this = self as *mut Self;
            unsafe {
                let mut inode = (*this).get_inode();
                (*this).data_blocks.push(new_blk as u32);
                inode.incr_data_blocks(1);
                // inode.save_data_blocks(&self.data_blocks);
                // inode.save();
                self.data_block_num = new_blk as u32;
                self.next_write_index = 0;
            }
            true
        } else {
            false
        }
    }

    pub fn get_inode_block(&mut self, i: usize) -> (u32, Block) {
        get_inode_block(self.inode_table, i)
    }

    pub fn get_inode(&mut self) -> InodeProxy {
        InodeProxy::new(self.fs_meta_data, self.inode_table, self.disk.clone(), self.inumber)
    }
    pub fn save_inode(&mut self) -> bool {
        let this = self as *mut Self;
        unsafe {
            let mut inode = (*this).get_inode();
            inode.save();
            inode.save_data_blocks(&self.data_blocks);
        }
        true
    }
}



pub struct InodeReadIter<'a> {
    inode_table: &'a mut Vec<(u32, InodeBlock)>,
    fs_meta_data: &'a mut MetaData,
    pub inumber: usize,
    pub disk: Disk<'a>,
    pub data_blocks: Vec<u32>,
    pub curr_data_block: Block,
    pub curr_block_index: usize,
    pub byte_offset: usize
}

impl<'a> InodeReadIter<'a> {
    pub fn new(
        inumber: usize, fs_meta_data: &'a mut MetaData, 
        inode_table: &'a mut Vec<(u32, InodeBlock)>,
        mut disk: Disk<'a>
    ) -> Self {
        let disc = disk.clone();
        let mut inode = InodeProxy::new(fs_meta_data, inode_table, disk.clone(), inumber);
        let direct_pointers = inode.to_direct_pointers();
        let mut curr_data_block = Block::new();

        if direct_pointers.len() > 0 {
            disk.read(direct_pointers[0] as usize, curr_data_block.data_as_mut());
        }

        InodeReadIter {
            fs_meta_data,
            inode_table,
            inumber,
            disk,
            data_blocks: direct_pointers,
            curr_data_block,
            curr_block_index: 0,
            byte_offset: 0
        }
    }

    pub fn seek(&mut self, offset: usize) {
        if offset as u32 <= self.get_inode().size() {
            self.byte_offset = offset;
        }
    }

    pub fn align_to_block(&mut self, block_data: &mut [u8]) -> (bool, i64) {
        let n = self.byte_offset % Disk::BLOCK_SIZE;
        if n == 0 {
            return (true, 0)
        }

        if block_data.len() < n {
            return (false, -1)
        }

        let mut i = 0;
        for byte in self {
            block_data[i] = byte;
            i += 1;
            if i > n {
                break
            }
        }
        (true, i as i64)
    }

    pub fn read_buffer(&mut self, block_data: &mut [u8], length: usize) -> i64 {
        let mut read_len = length; //block_data.len();
        let (success, num_bytes) = self.align_to_block(&mut block_data[..]);
        if !success {
            return -1;
        }
        read_len -= num_bytes as usize;
        let mut bytes_read = num_bytes;

        let num_blocks = (read_len as f64 / Disk::BLOCK_SIZE as f64).floor() as usize;
        let block_offset = (self.byte_offset as f64 / Disk::BLOCK_SIZE as f64).floor() as usize;

        let mut i = 0;  // i = 0, 1, 2, 3, 4, ... num_blocks
        let mut start = num_bytes as usize;
        let mut end = start + Disk::BLOCK_SIZE;
        loop {
            if block_offset + i >= self.data_blocks.len() || i == num_blocks {
                // println!("This is me: {}, {}, {}, {}", num_blocks, block_offset, i, self.data_blocks.len());
                break
            }
            let blk_num = self.data_blocks[block_offset + i];
            if blk_num > 0 {
                self.disk.read(blk_num as usize, &mut block_data[start..end]);
                start = end;
                end = start + Disk::BLOCK_SIZE;
                i += 1;
                bytes_read += Disk::BLOCK_SIZE as i64;
                self.byte_offset += Disk::BLOCK_SIZE;
            } else {
                break
            }
        }

        // spill over block
        let num_bytes = read_len % Disk::BLOCK_SIZE;
        println!("This is great: {}", num_bytes);

        if num_bytes == 0 {
            return bytes_read
        }
        let mut i = 0;
        for byte in self {
            block_data[i + end - Disk::BLOCK_SIZE] = byte;
            i += 1;
            bytes_read += 1;
            if i == num_bytes {
                break
            }
        }

        bytes_read as i64
    }

    pub fn get_inode(&mut self) -> InodeProxy {
        InodeProxy::new(self.fs_meta_data, self.inode_table, self.disk.clone(), self.inumber)
    }
}

// inode iter returns byte by byte
impl<'a> Iterator for InodeReadIter<'a> {
    type Item = u8;
    fn next(&mut self) -> Option<Self::Item> {
        if self.byte_offset as u32 == self.get_inode().size() {
            return None
        }

        let block_index = (self.byte_offset as f64 / Disk::BLOCK_SIZE as f64).floor() as usize;
        let block_offset = self.byte_offset % Disk::BLOCK_SIZE;

        if block_index == self.curr_block_index {
            self.byte_offset += 1;
            return Some(
                self.curr_data_block.data()[block_offset]
            )
        }
        
        if block_index < self.data_blocks.len() && self.data_blocks[block_index] > 0 {
            let mut block = Block::new();
            self.disk.read(self.data_blocks[block_index] as usize, block.data_as_mut());
            self.curr_data_block = block;
            self.curr_block_index += 1;
            self.byte_offset += 1;
            return Some(
                self.curr_data_block.data()[block_offset]
            )
        }
        None
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


// InodeList iterator keeps track of the current inodeBlock and is responsible for
// 1. adding new inodeBlock to the list when the current block is full
// 2. returning the next inode in the current inodeBlock.
// 3. iterating over each (file or) inode within the InodeList

pub struct InodeList<'a> {
    curr_inode_block: InodeBlock,
    fs_meta_data: &'a mut MetaData,
    inode_table: &'a mut Vec<(u32, InodeBlock)>,
    inode_index: usize,
    disk: Disk<'a>
}

impl<'a> InodeList<'a> {
    pub fn new(
        fs_meta_data: &'a mut MetaData, 
        inode_table: &'a mut Vec<(u32, InodeBlock)>,
        disk: Disk<'a>
    ) -> Self {
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
            inode_table,
            inode_index: i ,
            curr_inode_block: inode_blk
        }
    }

    pub fn add(&mut self, inode: Inode) -> i64 {
        if self.inodeblock_isfull() {
            -1
        } else {
            let inumber = self.fs_meta_data.superblock.inodes as usize;
            self.fs_meta_data.inodes_root_block
                .iblock_as_mut()
                .inodes[self.inode_index] = inode;
            self.inode_index += 1;
            self.fs_meta_data.superblock.inodes += 1;
            self.set_inode(inumber, inode);
            self.save_inode(inumber);
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

    pub fn get_inode_block(&mut self, i: usize) -> (u32, Block) {
        get_inode_block(self.inode_table, i)
    }

    pub fn get_inode(&mut self, inumber: usize) -> InodeProxy {
        InodeProxy::new(self.fs_meta_data, self.inode_table, self.disk.clone(), inumber)
    }

    pub fn set_inode(&mut self, inumber: usize, inode: Inode) {
        set_inode(self.inode_table, inumber, inode)
    }

    pub fn save_inode(&mut self, inumber: usize) -> bool {
        self.get_inode(inumber).save()
    }

    pub fn iter<'b:'a>(&'b mut self) -> InodeListIter<'b> {
        InodeListIter::new(
            self.fs_meta_data.superblock.inodes as usize,
            self.curr_inode_block,
            self.inode_table,
            self.disk.clone()
        )
    }

    pub fn read_iter<'b: 'a>(&'b mut self, inumber: usize) -> InodeReadIter<'b> {
        // use the inumber to fetch the inode
        // let mut inode = self.get_inode(inumber);
        InodeReadIter::new(inumber, self.fs_meta_data, self.inode_table, self.disk.clone())
    }

    pub fn write_iter<'b: 'a>(&'b mut self, inumber: usize) -> InodeWriteIter<'b> {
        InodeWriteIter::new(inumber, self.disk.clone(), self.fs_meta_data, self.inode_table)
    }
}

// ITERATOR FOR INODE LIST
pub struct InodeListIter<'b> {
    curr_inode_block: InodeBlock,
    next_i: usize,  // index used by the iterator
    total_items: usize,
    inode_table: &'b mut Vec<(u32, InodeBlock)>,
    iblock_index: usize,
    disk: Disk<'b>
}

impl<'b> InodeListIter<'b> {
    pub fn new(total_items: usize, start_inode_block: InodeBlock, 
        inode_table: &'b mut Vec<(u32, InodeBlock)>, 
        disk: Disk<'b>) -> Self {
        InodeListIter {
            total_items,
            curr_inode_block: start_inode_block,
            inode_table,
            iblock_index: 0,
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
            if self.iblock_index < self.inode_table.len() {
                let (blk_num,next_iblock) = self.inode_table[self.iblock_index];
                self.iblock_index += 1;
                let mut block = Block::new();
                self.curr_inode_block = next_iblock;
            }
            return self.get_next()
        } else {
            return self.get_next()
        }
        Some(Inode::blank())
    }
}