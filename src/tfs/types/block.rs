use crate::tfs::disk::Disk;
use crate::tfs::constants::*;
use crate::tfs::utility::*;
use super::inode::*;
use super::MetaData;

#[derive(Copy, Clone, Debug)]
#[allow(dead_code)]
pub struct Superblock {
    pub magic_number: u32,
    pub blocks: u32,
    pub current_block_index: u32,
    pub free_blocks: u32,
    pub free_inodes: u32,
    pub inodes: u32
}

#[derive(Copy, Clone, Debug)]
pub struct InodeBlock {
    pub inodes: [Inode; INODES_PER_BLOCK - 1],
    pub next_block: u32,  // 8 bytes +
    pub prev_block: u32,  // 8 bytes +
    pub temp: [u32; 12]   //  12 * 4 = 64 bytes => size of 1 Inode
}

#[derive(Copy, Clone)]
#[allow(dead_code)]
pub union Block {
    pub superblock: Superblock,
    pub inode_block: InodeBlock,
    pub pointers: [u32; POINTERS_PER_BLOCK],
    pub data: [u8; Disk::BLOCK_SIZE]
}

pub struct BlockManager<'a> {
    meta_data: &'a mut MetaData,
    disk: Disk<'a>
    // inode_table: Vec<(u32, InodeBlock)>
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

impl<'a> BlockManager<'a> {
    pub fn new(meta_data: &'a mut MetaData, disk: Disk<'a>) -> Self {
        BlockManager {
            meta_data,
            disk
        }
    }

    pub fn allocate_free_block<'b:'a>(&'b mut self) -> i64 {
        let this = self as *mut Self;

        unsafe {
            if self.meta_data.superblock.free_blocks == 0 {
                // allocate from current_block_index
                let free_blk = (*this).meta_data.superblock.current_block_index;
                if free_blk < (*this).meta_data.superblock.blocks {
                    (*this).meta_data.superblock.current_block_index += 1;
                    (*this).save_meta_data("superblock");   // save meta_data to disk here
                    return free_blk as i64;
                }
            } else {
                // read the head of the free_list
                let n = ((*this).meta_data.superblock.free_blocks - 1) as usize;
                let mut i = n % (POINTERS_PER_BLOCK - 1);
                let mut changed_root = false;
                let mut next_ptr = 0;
                
                if i == POINTERS_PER_BLOCK - 2 && 
                    (*this).meta_data.root_free_list.pointers_as_mut()[0] == 0 &&
                    (*this).meta_data.root_free_list.pointers_as_mut()[POINTERS_PER_BLOCK - 2] == 0 {
                    // change the root free list here
                    // println!("changing root list here..");
                    next_ptr = (*this).meta_data.root_free_list.pointers_as_mut()[POINTERS_PER_BLOCK - 1];
                    if next_ptr > 0 {
                        let mut block = fetch_block(&mut self.disk, next_ptr as usize);
                        (*this).meta_data.root_free_list = block;
                        changed_root = true;
                    }
                }
    
                let mut free_blk = 0;
                if changed_root {
                    free_blk = next_ptr;
                } else {
                    free_blk = (*this).meta_data.root_free_list.pointers_as_mut()[i];
                    (*this).meta_data.root_free_list.pointers_as_mut()[i] = 0;
                    (*this).meta_data.superblock.free_blocks = (*this).meta_data.superblock.free_blocks - 1;
                }
    
                // save and return free block
                (*this).save_meta_data("all");
                return free_blk as i64;
            }
        }
        -1
    }

    pub fn free_blocks<'b:'a>(&'b mut self, blocks: Vec<u32>) {
        // function to add to the free list
        println!("Adding {} free blocks", blocks.len());
        let this = self as *mut Self;

        unsafe {
            let mut disk = (*this).disk.clone();
            let mut free_block = (*this).meta_data.root_free_list.pointers();
            let mut i = 0;
            loop {
                if i == POINTERS_PER_BLOCK {
                    break
                }
                if (*this).meta_data.root_free_list.pointers_as_mut()[i] == 0 {
                    break
                }
                i += 1;
            }

            for b in blocks.iter() {
                if i >= POINTERS_PER_BLOCK - 1 {
                    let new_blk = *b;
                    if new_blk > 0 {
                        disk.write(new_blk as usize, (*this).meta_data.root_free_list.data_as_mut());
                        let mut blk = Block::new();
                        blk.pointers_as_mut()[POINTERS_PER_BLOCK - 1] = new_blk as u32;
                        (*this).meta_data.root_free_list.set_data(blk.data());
                        (*this).save_meta_data("rfl");
                        i = 0;
                    } else {
                        break
                    }
                } else {
                    (*this).meta_data.root_free_list.pointers_as_mut()[i] = *b;
                    (*this).meta_data.superblock.free_blocks += 1;
                    i += 1;
                }
            }

            (*this).save_meta_data("rfl");
            (*this).save_meta_data("superblock");
        }
    }

    pub fn save_meta_data<'b:'a>(&'b mut self, name: &'b str) -> bool {
        save_meta_data(self.meta_data, &mut self.disk, name)
    }
}