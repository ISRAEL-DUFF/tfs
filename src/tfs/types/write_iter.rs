
use crate::tfs::disk::Disk;
use crate::tfs::constants::*;
use super::{MetaData, InodeProxy, InodeBlock, Block};
use crate::tfs::utility::*;

pub struct InodeWriteIter<'a> {
    inumber: usize,
    inode: InodeProxy<'a>,
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
            let mut inode2 = InodeProxy::new(&mut (*meta_data), &mut (*i_table), disk.clone(), inumber);
            // let mut inode3 = InodeProxy::new(&mut (*meta_data), &mut (*i_table), disk.clone(), inumber);
            // let disc = disk.clone();
            let data_blocks = inode2.to_direct_pointers();
            // println!("DATABASL: {:?}", data_blocks);
            
            let mut i_write = InodeWriteIter {
                inumber,
                disk,
                buffer: [0; Disk::BLOCK_SIZE],
                data_blocks,
                inode,
                curr_data_block: Block::new(),
                data_block_num: 0,
                data_block_index: 0,
                next_write_index: 0,
                flushed: true,
                aligned_to_block: false,
                inode_table: &mut (*i_table),
                fs_meta_data: &mut (*meta_data)
            };

            i_write.seek_to_end()            

            // InodeWriteIter {
            //     inumber,
            //     disk,
            //     buffer: [0; Disk::BLOCK_SIZE],
            //     data_blocks,
            //     inode,
            //     curr_data_block: Block::new(),
            //     data_block_num: 0,
            //     data_block_index: 0,
            //     next_write_index: 0,
            //     flushed: false,
            //     aligned_to_block: false,
            //     inode_table: &mut (*i_table),
            //     fs_meta_data: &mut (*meta_data)
            // }
        }
    }

    pub fn seek<'h: 'a>(&'h mut self, offset: usize) {
        let this = self as *mut Self;
        let inode = unsafe {(*this).get_inode()};
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

    pub fn seek_to_end<'f:'a>(mut self) -> Self {
        let this = &mut self as *mut Self;
        let inode = unsafe {(*this).get_inode()};
        let offset = inode.size() as usize;
        inode.init_datablocks();

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

        self
    } 

    pub fn has_datablock<'h: 'a>(&'h mut self) -> bool {
        // println!("Has databalock: {}", self.get_inode().data_block());
        if self.get_inode().data_block() > 0 {
            true
        } else {
            false
        }
    }

    pub fn set_data_block<'h: 'a>(&'h mut self, blk_num: i64) -> bool {
        if blk_num > 0 {
            let this = self as *mut Self;
            let mut inode = unsafe{(*this).get_inode()};
            inode.set_data_block(blk_num as u32);
            self.save_inode();
            true
        } else {
            false
        }
    }

    pub fn write_byte<'h:'a>(&'h mut self, byte: u8) -> (bool, i64) {
        let this = self as *mut Self;
        let data_blocks = unsafe {(*this).get_inode().data_blocks()};
        if data_blocks.len() == 0 {
            return (false, -1)
        }
        if self.next_write_index < Disk::BLOCK_SIZE {
            self.curr_data_block.data_as_mut()[self.next_write_index] = byte;
            self.next_write_index += 1;
            self.aligned_to_block = false;
            self.flushed = false;
            (true, 1)
        } else {
            // flush current block
            if !self.flush() {
                println!("No flush {}", data_blocks.len());
                return (false, -1);
            };

            // add another block as current block
            if self.data_block_index < data_blocks.len() - 1 {
                self.data_block_index += 1;
                self.data_block_num = data_blocks[self.data_block_index];
                let mut data_block = Block::new();
                self.disk.read(self.data_block_num.clone() as usize, data_block.data_as_mut());
                self.curr_data_block = data_block;

                self.curr_data_block.data_as_mut()[self.next_write_index] = byte;
                self.next_write_index += 1;
                self.aligned_to_block = false;
                self.flushed = false;
                return (true, 1)
            } else {    // add new data block
                self.data_block_index += 1;
                (false, -1)
            }
        }
    }

    // the align_to_block() method is used to align the write position to the beginning of a new block
    // this enables the use of write_block() method.
    pub fn align_to_block<'h: 'a>(&'h mut self, block_data: &mut [u8]) -> (bool, i64) {
        if self.aligned_to_block {
            return (true, 0)
        }

        let n = Disk::BLOCK_SIZE - self.next_write_index;
        if block_data.len() < n {
            return (false, -1)
        }
        let this = self as *mut Self;
        if unsafe{(*this).get_inode().size()} == 0 || self.next_write_index == 0 || self.next_write_index == Disk::BLOCK_SIZE {
            self.aligned_to_block = true;
            return (true, 0)
        }

        let mut m = 0;
        loop {
            let (success, _) = unsafe{(*this).write_byte(block_data[m])};
            if !success {
                break
            }
            m += 1;
        }
        self.aligned_to_block = true;
        (true, m as i64)
    }

    pub fn write_block<'h: 'a>(&'h mut self, block_num: i64, block_data: &mut [u8]) -> i64 {
        if block_num < 0  || !self.aligned_to_block {
            return -1
        }
        if block_data.len() < Disk::BLOCK_SIZE {
            return -1
        }

        let r = self.add_data_blk(block_num);

        if r != 0 {
            return r
        }

        self.disk.write(block_num as usize, block_data);
        self.aligned_to_block = true;

        let mut inode = self.get_inode();
        inode.incr_size(Disk::BLOCK_SIZE);
        // inode.save();
        0
    }

    pub fn flush(&mut self) -> bool {
        // write current block to disk
        // increase file size and reset next write index
        let this = self as *mut Self;
        let data_blocks = unsafe {(*this).get_inode().data_blocks()};
        if self.data_block_num > 0 && data_blocks.len() > 0 {
            unsafe {
                if !self.aligned_to_block && !self.flushed {
                    // println!("Writing to block: {}", self.data_block_num);
                    (*this).disk.write(self.data_block_num as usize, self.curr_data_block.data_as_mut());
                    (*this).get_inode().incr_size(self.next_write_index);
                    // println!("Wrote to block: {}", self.data_block_num);
                    println!("DATABlooooKs :{:?}", data_blocks.len());
                 } 
                
                if (*this).next_write_index == Disk::BLOCK_SIZE {
                    (*this).next_write_index = 0;
                }
                (*this).flushed = true;
                (*this).save_inode();
            }
            true
        } else {
            false
        }
    }

    pub fn add_data_blk(&mut self, new_blk: i64) -> i64 {
        println!("Adding block: {}", new_blk);
        if new_blk > 0 {
            let this = self as *mut Self;
            unsafe {
                let mut inode = (*this).get_inode();
                (*this).data_blocks.push(new_blk as u32); // not needed again
                let r = inode.add_data_block(new_blk);
                // inode.incr_data_blocks(1);

                // inode.save_data_blocks(&self.data_blocks);
                // inode.save();
                self.data_block_num = new_blk as u32;
                self.next_write_index = 0;
                self.curr_data_block = Block::new();
                println!("RETun: {}", r);
                return r;
            }
        } else {
            return -1;
        }
    }

    pub fn get_inode_block(&mut self, i: usize) -> (u32, Block) {
        get_inode_block(self.inode_table, i)
    }

    pub fn get_inode<'h: 'a>(&'h mut self) -> &mut InodeProxy {
        // this method is now too expensive because of InodeDataPointer inside InodeProxy struct
        //InodeProxy::new(self.fs_meta_data, self.inode_table, self.disk.clone(), self.inumber)

        &mut self.inode
    }
    pub fn save_inode(&mut self) -> bool {
        let this = self as *mut Self;
        unsafe {
            let mut inode = (*this).get_inode();
            inode.save();
            // inode.save_data_blocks(&self.data_blocks);
            inode.save_data_blocks();
            // println!("INDO: {:?}", inode.data_blocks());
        }
        true
    }
}