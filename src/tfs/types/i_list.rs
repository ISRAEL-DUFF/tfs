
use crate::tfs::disk::Disk;
use crate::tfs::constants::*;
use super::{MetaData, InodeReadIter, InodeWriteIter};
use super::block::{Block, BlockManager, InodeBlock};
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
        if self.fs_meta_data.superblock.free_inodes == 0 {
            let this = self as *mut Self;
            unsafe {
                if (*this).inodeblock_isfull() {
                    let mut block_mgr = BlockManager::new((*this).fs_meta_data, (*this).disk.clone());
                    let new_blk = block_mgr.allocate_free_block();
                    if !(*this).add_inodeblock(new_blk) {
                        return -1;
                    }
                }
                self.fs_meta_data.superblock.inodes += 1;
                let inumber = self.fs_meta_data.superblock.inodes as usize;
                self.fs_meta_data.inodes_root_block
                    .iblock_as_mut()
                    .inodes[self.inode_index] = inode;
                self.inode_index += 1;
                // self.fs_meta_data.superblock.inodes += 1;
                self.set_inode(inumber, inode);
                self.save_inode(inumber);
                println!("INUMBER I: {}", inumber);
                inumber as i64
            }
        } else {
            let mut meta_data = &mut self.fs_meta_data;
            // read the head of the free_list
            let n = meta_data.superblock.free_inodes as usize;
            let mut i = n % (POINTERS_PER_BLOCK - 1);

            // println!("InodeList ADd: {}, {}, {:?}", i, n, meta_data.inodes_free_list.pointers());
            println!("InodeList ADd: {}, {}", i, n);

            if i == POINTERS_PER_BLOCK - 2 && 
                meta_data.inodes_free_list.pointers_as_mut()[0] == 0 &&
                meta_data.inodes_free_list.pointers_as_mut()[POINTERS_PER_BLOCK - 2] == 0 {
                // change the root free list here
                // println!("changing root list here..");
                let next_ptr = meta_data.inodes_free_list.pointers_as_mut()[POINTERS_PER_BLOCK - 1];
                if next_ptr > 0 {
                    let mut block = fetch_block(&mut self.disk, next_ptr as usize);
                    meta_data.inodes_free_list = block;
                } else {
                    return -1;
                }
            }

            let mut free_inum = 0;
            free_inum = meta_data.inodes_free_list.pointers_as_mut()[i-1];
            meta_data.inodes_free_list.pointers_as_mut()[i-1] = 0;
            meta_data.superblock.free_inodes -= 1;

            self.set_inode(free_inum as usize, Inode::blank());
            self.save_inode(free_inum as usize);
            println!("INUMBER II: {}", free_inum);
            return free_inum as i64;
        }
    }

    pub fn remove(&mut self, inumber: usize) -> bool {
        let this = self as *mut Self;
        unsafe {
            let mut inode = (*this).get_inode(inumber);
            let mut block_mgr = BlockManager::new((*this).fs_meta_data, (*this).disk.clone());
            let mut block_manager = &mut block_mgr as *mut BlockManager;

            // free data blocks
            (*block_manager).free_blocks(inode.deallocate_ptrs());

            let mut i = 0;
            loop {
                if i == POINTERS_PER_BLOCK {
                    break
                }
                if (*this).fs_meta_data.inodes_free_list.pointers_as_mut()[i] == 0 {
                    break
                }
                i += 1;
            }

            if i >= POINTERS_PER_BLOCK - 1 {
                let new_blk = (*block_manager).allocate_free_block();
                if new_blk > 0 {
                    (*this).disk.write(new_blk as usize, (*this).fs_meta_data.inodes_free_list.data_as_mut());
                    let mut blk = Block::new();
                    blk.pointers_as_mut()[POINTERS_PER_BLOCK - 1] = new_blk as u32;
                    (*this).fs_meta_data.inodes_free_list.set_data(blk.data());
                    (*block_manager).save_meta_data("rfl");
                    i = 0;
                }
            }
            (*this).fs_meta_data.inodes_free_list.pointers_as_mut()[i] = inumber as u32;
            (*this).fs_meta_data.superblock.free_inodes += 1;

            (*block_manager).save_meta_data("rfl");
            (*block_manager).save_meta_data("superblock");
        }
        true
    }

    pub fn inodeblock_isfull(&self) -> bool {
        if self.inode_index == INODES_PER_BLOCK - 1 {
            true
        } else {
            false
        }
    }

    pub fn inode_exists(&mut self, inumber: usize) -> bool {
        let inrange = inumber > 0 && (inumber - 1) < self.fs_meta_data.superblock.inodes as usize;
        if inrange {
            self.get_inode(inumber).is_valid()
        } else {
            false
        }
    }

    pub fn add_inodeblock(&mut self, new_blk: i64) -> bool {
        if new_blk > 0 {
            let mut inode_blk = InodeBlock::new();
            inode_blk.next_block = new_blk as u32;
            let mut block = inode_blk.as_block();

            // update disk data
            let mut root_blk = self.fs_meta_data.inodes_root_block;
            root_blk.iblock_as_mut().prev_block = new_blk as u32;
            self.disk.write(new_blk as usize, root_blk.data_as_mut());
            self.disk.write(INODES_ROOT_BLOCK, block.data_as_mut());
            self.fs_meta_data.inodes_root_block = block;
            self.inode_index = 0;

            // update in-memory data
            let mut r = self.inode_table.pop().unwrap();
            r.0 = new_blk as u32;
            r.1 = root_blk.inode_block();
            self.inode_table.push(r);
            self.inode_table.push((INODES_ROOT_BLOCK as u32, self.fs_meta_data.inodes_root_block.inode_block()));

            // panic!("Detail: {:?}", self.inode_table);
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