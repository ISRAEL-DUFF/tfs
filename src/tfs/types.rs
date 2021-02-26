// mod disk;
use super::disk::Disk;
// use super::utility;

pub const MAGIC_NUMBER: usize = 0xf0f03410;
pub const INODES_PER_BLOCK: usize   = 128;
pub const POINTERS_PER_INODE: usize = 5;
pub const POINTERS_PER_BLOCK: usize = 1024;

#[derive(Copy, Clone, Debug)]
#[allow(dead_code)]
pub struct Superblock {
    pub magic_number: u32,
    pub blocks: u32,
    pub inode_blocks: u32,
    pub inodes: u32
}

#[derive(Copy, Clone, Debug)]
#[allow(dead_code)]
pub struct Inode {
    pub valid: u32, // whether or not inode is valid (or allocated)
    pub size: u32,  // size of file
    pub direct: [u32; POINTERS_PER_INODE],
    pub single_indirect: u32,
    pub double_indirect: u32,
    pub triple_indirect: u32
}

#[derive(Copy, Clone)]
#[allow(dead_code)]
pub union Block {
    pub superblock: Superblock,
    pub inodes: [Inode; INODES_PER_BLOCK],
    pub pointers: [u32; POINTERS_PER_BLOCK],
    pub data: [u8; Disk::BLOCK_SIZE]
}

// #[derive(Copy, Clone, Debug)]
#[allow(dead_code)]
pub struct MetaData {
    pub superblock: Superblock,
    pub inode_table: Vec<[Inode; INODES_PER_BLOCK]>
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

    pub fn superblock(&self) -> Superblock {
        unsafe {
            self.superblock
        }
    }

    pub fn inodes(&self) -> [Inode; INODES_PER_BLOCK] {
        unsafe {
            self.inodes
        }
    }

    pub fn pointers(&self) -> [u32; POINTERS_PER_BLOCK]{
        unsafe {
            self.pointers
        }
    }

    pub fn set_data(&mut self, data: [u8; Disk::BLOCK_SIZE]) {
        self.data = data;
    }

    pub fn set_inodes(&mut self, inodes: [Inode; INODES_PER_BLOCK]) {
        self.inodes = inodes;
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
            valid: 0,
            size: 0,
            direct: [0; POINTERS_PER_INODE],
            single_indirect: 0,
            double_indirect: 0,
            triple_indirect: 0
        }
    }
}
