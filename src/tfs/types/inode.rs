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
    pub size: u64,  // size of file
    pub ctime: u32,
    pub atime: u32,
    pub blk_pointer_level: u32, // level 1 = direct, level 2 = single_indirect, level 3 = double_indirect etc.
    pub data_block: u32,    // pointer to the data blocks
    pub total_data_blocks: u32,  // total number of data blocks allocated to this inode
    pub hard_links: u32,
    pub user_id: u32,
    pub group_id: u32,
    pub mode: u32,
    pub kind: u32,  // regular file = 1, directory = 2
    temp2: u32,
    temp3: u32,
    temp4: u32
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
        let mut inode = Self::new();
        inode.valid = 1;
        inode
    }

    pub fn new() -> Self {
        Inode {
            valid: 0,
            size: 0,
            atime: 0,
            ctime: 0,
            blk_pointer_level: 0,  // start from direct pointers
            total_data_blocks: 0,
            data_block: 0,
            hard_links: 0,
            user_id: 0,
            group_id: 0,
            mode: 0,
            kind: 0,
            temp2: 0,
            temp3: 0,
            temp4: 0
        }
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

    pub fn size(&self) -> u64 {
        let (i, j) = self.get_index();
        self.inode_table[i].1.inodes[j].size
    }

    pub fn data_block(&self) -> u32 {
        let (i, j) = self.get_index();
        self.inode_table[i].1.inodes[j].data_block
    }

    pub fn total_data_blocks(&self) -> u32 {
        let (i, j) = self.get_index();
        self.inode_table[i].1.inodes[j].total_data_blocks
    }

    pub fn data_blocks<'d:'c>(&'d mut self) -> &Vec<u32> {
       let this = self as *mut Self;
       let data_manager = match &self.data_manager {
            Some(data_manager) => {
                data_manager
            },
            _ => {
                let manager = InodeDataPointer::with_depth(self.pointer_level() as usize, self.disk.clone(), unsafe{(*this).data_block()});
               unsafe{ (*this).data_manager = Some(manager); }
                match &self.data_manager {
                    Some(m) => m,
                    _ => panic!("Unable to create inode data manager")
                }
            }
        };
        &data_manager.direct_pointers()
    }

    pub fn data_manager<'d: 'c>(&'d self) -> Option<&'d InodeDataPointer> {
        self.data_manager.as_ref()
    }

    pub fn data_manager_mut<'d: 'c>(&'d mut self) -> Option<&'d mut InodeDataPointer> {
        self.data_manager.as_mut()
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

    pub fn init_datablocks<'g:'c>(&'g mut self) -> bool {
        if self.data_block() == 0 {
            return false;
        }
        let b = self.data_block();
        let manager = InodeDataPointer::with_depth(self.pointer_level() as usize, self.disk.clone(), b);
        self.data_manager = Some(
            manager
        );
        true
    }

    pub fn incr_size(&mut self, amount: usize) {
        let (i, j) = self.get_index();
        self.inode_table[i].1.inodes[j].size += amount as u64;
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
        let d = data_manager.depth() as u32;
        let (i, j) = unsafe {(*this).get_index()};
        self.inode_table[i].1.inodes[j].blk_pointer_level = d;
        if r == 0 {
            unsafe{(*this).incr_data_blocks(1)};
        }
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

    pub fn save_data_blocks(&mut self) {
        match &mut self.data_manager {
            Some(data_manager) => {
                data_manager.save();
            },
            _ => {}
        };
    }

    pub fn to_direct_pointers<'g:'c>(&'g mut self) -> Vec<u32> {
        let b = self.data_block();
        if b > 0 {
            InodeDataPointer::with_depth(self.pointer_level() as usize, self.disk.clone(), b)
            .blocks()
        } else {
            Vec::new()
        }
    }

    pub fn deallocate_ptrs(mut self) -> Vec<u32> {
        let this = &mut self as *mut Self;
        let mut ptrs = vec![];
        unsafe {(*this).init_datablocks()};

        let (i, j) = self.get_index();
        self.inode_table[i].1.inodes[j].data_block = 0;
        self.inode_table[i].1.inodes[j].valid = 0;
        self.inode_table[i].1.inodes[j].total_data_blocks = 0;
        self.inode_table[i].1.inodes[j].blk_pointer_level = 0;
        self.inode_table[i].1.inodes[j].size = 0;
        unsafe {(*this).save()};
        
        match self.data_manager {
            Some(manager) => {
                // free root block
                ptrs.push(manager.root_ptr);

                // next, free indirect pointers
                for ptr in manager.indirect_ptrs.iter() {
                    for p in ptr.iter() {
                        ptrs.push(*p);
                    }
                }

                // free direct pointers
                for ptr in manager.direct_ptrs.iter() {
                    ptrs.push(*ptr);
                }

            },
            _ => {}
        };
        ptrs
    }
}