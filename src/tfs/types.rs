// mod disk;
use super::disk::Disk;
use super::utility;

pub const MAGIC_NUMBER: usize = 0xf0f03410;
pub const INODES_PER_BLOCK: usize   = 128;
pub const POINTERS_PER_INODE: usize = 5;
pub const POINTERS_PER_BLOCK: usize = 1024;

#[derive(Copy, Clone, Debug)]
#[allow(dead_code)]
pub struct Superblock {
    pub MagicNumber: u32,
    pub Blocks: u32,
    pub InodeBlocks: u32,
    pub Inodes: u32
}

#[derive(Copy, Clone, Debug)]
#[allow(dead_code)]
pub struct Inode {
    pub Valid: u32, // whether or not inode is valid (or allocated)
    pub Size: u32,  // size of file
    pub Direct: [u32; POINTERS_PER_INODE],
    pub Indirect: u32
}

#[derive(Copy, Clone)]
#[allow(dead_code)]
pub union Block {
    pub Super: Superblock,
    pub Inodes: [Inode; INODES_PER_BLOCK],
    pub Pointers: [u32; POINTERS_PER_BLOCK],
    pub Data: [u8; Disk::BLOCK_SIZE]
}

// #[derive(Copy, Clone, Debug)]
#[allow(dead_code)]
pub struct MetaData {
    pub superBlock: Superblock,
    pub inodeTable: Vec<[Inode; INODES_PER_BLOCK]>
}


#[allow(dead_code)]
impl Block {
    pub fn new() -> Self {
        Block {
            Data: [0; Disk::BLOCK_SIZE]
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
            self.Data
        }
    }

    pub fn superblock(&self) -> Superblock {
        unsafe {
            self.Super
        }
    }

    pub fn inodes(&self) -> [Inode; INODES_PER_BLOCK] {
        unsafe {
            self.Inodes
        }
    }

    pub fn pointers(&self) -> [u32; POINTERS_PER_BLOCK]{
        unsafe {
            self.Pointers
        }
    }

    pub fn set_data(&mut self, data: [u8; Disk::BLOCK_SIZE]) {
        unsafe {
            self.Data = data;
        }
    }

    pub fn set_inodes(&mut self, inodes: [Inode; INODES_PER_BLOCK]) {
        unsafe {
            self.Inodes = inodes;
        }
    }

    pub fn set_pointers(&mut self, pointers: [u32; POINTERS_PER_BLOCK]) {
        unsafe {
            self.Pointers = pointers;
        }
    }

    pub fn set_superblock(&mut self, superblock: Superblock) {
        unsafe {
            self.Super = superblock;
        }
    }
}

impl Inode {
    pub fn blank() -> Self {
        Inode {
            Valid: 0,
            Size: 0,
            Direct: [0; POINTERS_PER_INODE],
            Indirect: 0
        }
    }
}
