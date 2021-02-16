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
}
