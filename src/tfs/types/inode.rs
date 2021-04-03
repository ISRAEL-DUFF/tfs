use crate::tfs::disk::Disk;
use crate::tfs::constants::*;
use super::MetaData;
use super::{Block, InodeBlock};
use super::data_ptr::*;
use crate::tfs::utility::*;

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
    data_manager: Option<InodeDataPointer<'c>>,
    inode_table: &'c mut Vec<(u32, InodeBlock)>,
    fs_meta_data: &'c mut MetaData,
    pub disk: Disk<'c>
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


// the inode proxy
impl<'c> InodeProxy<'c> {
    pub fn new(fs_meta_data: &'c mut MetaData,
        inode_table: &'c mut Vec<(u32, InodeBlock)>, 
        disk: Disk<'c>, 
        inumber: usize
    ) -> Self {
        InodeProxy {
            inumber,
            data_manager: None,
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

    pub fn data_blocks<'d:'c>(&'d mut self) -> &Vec<u32> {
       let this = self as *mut Self;
       let data_manager = match &self.data_manager {
            Some(data_manager) => {
                data_manager
            },
            _ => {
                let manager = InodeDataPointer::new(self.disk.clone(), unsafe{(*this).data_block()});
               unsafe{ (*this).data_manager = Some(manager); }
                match &self.data_manager {
                    Some(m) => m,
                    _ => panic!("Unable to create inode data manager")
                }
            }
        };
        &data_manager.direct_pointers()
    }

    pub fn pointer_level(&self) -> u32 {
        let (i, j) = self.get_index();
        self.inode_table[i].1.inodes[j].blk_pointer_level
    }

    pub fn set_data_block<'f:'c>(&'f mut self, blk: u32) {
        let (i, j) = self.get_index();
        self.inode_table[i].1.inodes[j].data_block = blk;
        self.data_manager = Some(
            InodeDataPointer::new(self.disk.clone(), blk)
        );
    }

    pub fn init_datablocks<'g:'c>(&'g mut self) {
        if self.data_block() == 0 {
            return
        }
        let b = self.data_block();
        let manager = InodeDataPointer::with_depth(self.pointer_level() as usize, self.disk.clone(), b);
        self.data_manager = Some(
            manager
        );
    }

    pub fn incr_size(&mut self, amount: usize) {
        let (i, j) = self.get_index();
        self.inode_table[i].1.inodes[j].size += amount as u32;
    }

    pub fn incr_data_blocks(&mut self, amount: usize) {
        let (i, j) = self.get_index();
        self.inode_table[i].1.inodes[j].total_data_blocks += amount as u32;
    }

    pub fn add_data_block<'h: 'c>(&'h mut self, blk: i64) -> i64 {
        let this = self as *mut Self;
       let data_manager = match &mut self.data_manager {
            Some(data_manager) => {
                data_manager
            },
            _ => {
                let manager = InodeDataPointer::new(self.disk.clone(), unsafe{(*this).data_block()});
                self.data_manager = Some(
                    manager
                );
                match &mut self.data_manager {
                    Some(m) => m,
                    _ => return -1
                }
            }
        };

        let r = data_manager.add_data_block(blk);
        let d = (data_manager.depth() + 1 ) as u32;
        let (i, j) = unsafe {(*this).get_index()};
        self.inode_table[i].1.inodes[j].blk_pointer_level = d;
        unsafe{(*this).incr_data_blocks(1)};
        return r;
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


    // pub fn save_data_blocks(&mut self, data_blocks: &Vec<u32>) {
    //     // println!("Pointer Data Block: {}, level: {}, {}", self.data_block(), self.pointer_level(), data_blocks.len());
    //     let root_block = fetch_block(&mut self.disk.clone(), self.data_block() as usize);
    //     match self.pointer_level() {
    //         1 => {
    //             // println!("1st level");
    //             let mut i = 0;
    //             let mut direct_pointers = Block::new();
    //             for j in 0..root_block.pointers().len() {
    //                 if i < data_blocks.len() && data_blocks[i] > 0 {
    //                     direct_pointers.pointers_as_mut()[j as usize] = data_blocks[i];
    //                     i += 1;
    //                 } else {
    //                     break
    //                 }
    //             }
    //             if self.data_block() > 0 {
    //                 self.disk.write(self.data_block() as usize, direct_pointers.data_as_mut());
    //             }
    //         },
    
    //         2 => {
    //             // println!("2nd level");
    //             let mut i = 0;
    //             for ptr1 in root_block.pointers().iter() {
    //                 if *ptr1 > 0 {
    //                     let mut direct_pointers = Block::new();
    //                     for j in 0..POINTERS_PER_BLOCK {
    //                         if i < data_blocks.len() {
    //                             direct_pointers.pointers_as_mut()[j] = data_blocks[i];
    //                             i += 1;
    //                         } else {
    //                             break
    //                         }
    //                     }
    //                     self.disk.write(*ptr1 as usize, direct_pointers.data_as_mut());
    //                 } else {
    //                     break
    //                 }
    //             }
    //         },
    
    //         3 => {
    //             let mut i = 0;
    //             for ptr1 in root_block.pointers().iter() {
    //                 if *ptr1 > 0 {
    //                     let root_2_blocks = fetch_block(&mut self.disk, *ptr1 as usize);
    //                     for ptr2 in root_2_blocks.pointers().iter() {
    //                         if *ptr2 > 0 {
    //                             let mut direct_pointers = Block::new();
    //                             for j in 0..POINTERS_PER_BLOCK {
    //                                 if i < data_blocks.len() {
    //                                     direct_pointers.pointers_as_mut()[j] = data_blocks[i];
    //                                     i += 1;
    //                                 } else {
    //                                     break
    //                                 }
    //                             }
    //                             self.disk.write(*ptr2 as usize, direct_pointers.data_as_mut());
    //                         } else {
    //                             break
    //                         }
    //                     }
    //                 } else {
    //                     break
    //                 }
    //             }
    //         },
    
    //         _ => {}
    //     }
    // }

    pub fn save_data_blocks(&mut self) {
        match &mut self.data_manager {
            Some(data_manager) => {
                data_manager.save();
            },
            _ => {}
        };
    }

    // fn to_direct_pointers(&mut self) -> Vec<u32> {
    //     let data_block = self.data_block();
    //     let index_zero = |array: [u32; POINTERS_PER_BLOCK]| {
    //         let mut i = 0;
    //         loop {
    //             if i == array.len() || array[i] == 0 {
    //                 break
    //             }
    //             i += 1;
    //         }
    //         i
    //     };
    //     match self.pointer_level() {
    //         1 => {  // single indirect pointers
    //             if data_block == 0 {
    //                 return vec![]
    //             }
    //             let mut block = Block::new();
    //             self.disk.read(self.data_block() as usize, block.data_as_mut());
    //             let i = index_zero(block.pointers());
    //             Vec::from(&block.pointers()[0..i])
    //         },
    //         2 => {
    //             if data_block == 0 {
    //                 return vec![]
    //             }
    //             let mut block = Block::new();
    //             self.disk.read(self.data_block() as usize, block.data_as_mut());

    //             let mut data_blks = Block::new();
    //             let mut data_blocks = Vec::new();
    //             for blk in block.pointers().iter() {
    //                 if *blk == 0 {
    //                     break
    //                 }
    //                 self.disk.read(*blk as usize, data_blks.data_as_mut());
    //                 let i = index_zero(data_blks.pointers());
    //                 data_blocks.extend_from_slice(&data_blks.pointers()[..i]);
    //             }
    //             data_blocks
    //         },
    //         3 => {
    //             if data_block == 0 {
    //                 return vec![]
    //             }
    //             let mut block = Block::new();
    //             self.disk.read(self.data_block() as usize, block.data_as_mut());

    //             let mut data_blks_1 = Block::new();
    //             let mut data_blks_2 = Block::new();
    //             let mut data_blocks = Vec::new();
    //             for blk in block.pointers().iter() {
    //                 if *blk == 0 {
    //                     break
    //                 }
    //                 self.disk.read(*blk as usize, data_blks_1.data_as_mut());
    //                 for b in data_blks_1.pointers().iter() {
    //                     if *b == 0 {
    //                         break
    //                     }
    //                     self.disk.read(*b as usize, data_blks_2.data_as_mut());
    //                     let i = index_zero(data_blks_2.pointers());
    //                     data_blocks.extend_from_slice(&data_blks_2.pointers()[..i]);
    //                 }
    //             }
    //             data_blocks
    //         },
    //         _ => vec![]
    //     }
    // }

    pub fn to_direct_pointers<'g:'c>(&'g mut self) -> Vec<u32> {
        let b = self.data_block();
        if b > 0 {
            InodeDataPointer::with_depth(self.pointer_level() as usize, self.disk.clone(), b)
            .blocks()
        } else {
            Vec::new()
        }
    }
}