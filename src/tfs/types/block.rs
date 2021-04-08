use crate::tfs::disk::Disk;
use crate::tfs::constants::*;
use super::inode::*;

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
pub struct InodeBlock {
    pub inodes: [Inode; INODES_PER_BLOCK - 1],
    pub next_block: u32,  // 4 bytes +
    pub prev_block: u32,  // 4 bytes +
    pub temp: [u32; 14]   //  14 * 4 = 32 bytes => size of 1 Inode
}

#[derive(Copy, Clone)]
#[allow(dead_code)]
pub union Block {
    pub superblock: Superblock,
    pub inode_block: InodeBlock,
    pub pointers: [u32; POINTERS_PER_BLOCK],
    pub data: [u8; Disk::BLOCK_SIZE]
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