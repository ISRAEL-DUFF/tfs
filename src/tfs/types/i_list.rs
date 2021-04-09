
use crate::tfs::disk::Disk;
use crate::tfs::constants::*;
use super::{MetaData, InodeReadIter, InodeWriteIter};
use super::block::{Block, InodeBlock};
use super::inode::{Inode, InodeProxy};
use crate::tfs::utility::*;

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

    pub fn remove(&mut self, inumber: usize) -> Vec<u32> {
        let mut inode = self.get_inode(inumber);
        inode.deallocate_ptrs()
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