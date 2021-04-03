use crate::tfs::disk::Disk;
use crate::tfs::constants::*;
use super::{MetaData, InodeProxy, InodeBlock, Block};
use super::data_ptr::*;
use crate::tfs::utility::*;

pub struct InodeReadIter<'a> {
    inode_table: &'a mut Vec<(u32, InodeBlock)>,
    fs_meta_data: &'a mut MetaData,
    pub inumber: usize,
    pub disk: Disk<'a>,
    pub data_blocks: Vec<u32>,
    pub curr_data_block: Block,
    pub curr_block_index: usize,
    pub byte_offset: usize
}

impl<'a> InodeReadIter<'a> {
    pub fn new(
        inumber: usize, fs_meta_data: &'a mut MetaData, 
        inode_table: &'a mut Vec<(u32, InodeBlock)>,
        mut disk: Disk<'a>
    ) -> Self {
        let disc = disk.clone();
        let mut inode = InodeProxy::new(fs_meta_data, inode_table, disk.clone(), inumber);
        // let direct_pointers = inode.to_direct_pointers();
        let direct_pointers = inode.to_direct_pointers();
        let mut curr_data_block = Block::new();

        println!("DATABLOCKS: {:?}", direct_pointers);

        if direct_pointers.len() > 0 {
            disk.read(direct_pointers[0] as usize, curr_data_block.data_as_mut());
        }

        InodeReadIter {
            fs_meta_data,
            inode_table,
            inumber,
            disk,
            data_blocks: direct_pointers,
            curr_data_block,
            curr_block_index: 0,
            byte_offset: 0
        }
    }

    pub fn seek(&mut self, offset: usize) {
        if offset as u32 <= self.get_inode().size() {
            self.byte_offset = offset;
        }
    }

    pub fn align_to_block(&mut self, block_data: &mut [u8]) -> (bool, i64) {
        let n = self.byte_offset % Disk::BLOCK_SIZE;
        if n == 0 {
            return (true, 0)
        }

        if block_data.len() < n {
            return (false, -1)
        }

        let mut i = 0;
        for byte in self {
            block_data[i] = byte;
            i += 1;
            if i > n {
                break
            }
        }
        (true, i as i64)
    }

    pub fn read_buffer(&mut self, block_data: &mut [u8], length: usize) -> i64 {
        let mut read_len = length; //block_data.len();
        let (success, num_bytes) = self.align_to_block(&mut block_data[..]);
        if !success {
            return -1;
        }
        read_len -= num_bytes as usize;
        let mut bytes_read = num_bytes;

        let num_blocks = (read_len as f64 / Disk::BLOCK_SIZE as f64).floor() as usize;
        let block_offset = (self.byte_offset as f64 / Disk::BLOCK_SIZE as f64).floor() as usize;

        let mut i = 0;  // i = 0, 1, 2, 3, 4, ... num_blocks
        let mut start = num_bytes as usize;
        let mut end = start + Disk::BLOCK_SIZE;
        loop {
            if block_offset + i >= self.data_blocks.len() || i == num_blocks {
                // println!("This is me: {}, {}, {}, {}", num_blocks, block_offset, i, self.data_blocks.len());
                break
            }
            let blk_num = self.data_blocks[block_offset + i];
            if blk_num > 0 {
                self.disk.read(blk_num as usize, &mut block_data[start..end]);
                start = end;
                end = start + Disk::BLOCK_SIZE;
                i += 1;
                bytes_read += Disk::BLOCK_SIZE as i64;
                self.byte_offset += Disk::BLOCK_SIZE;
            } else {
                break
            }
        }

        // spill over block
        let num_bytes = read_len % Disk::BLOCK_SIZE;
        println!("This is great: {}", num_bytes);

        if num_bytes == 0 {
            return bytes_read
        }
        let mut i = 0;
        for byte in self {
            block_data[i + end - Disk::BLOCK_SIZE] = byte;
            i += 1;
            bytes_read += 1;
            if i == num_bytes {
                break
            }
        }

        bytes_read as i64
    }

    pub fn get_inode(&mut self) -> InodeProxy {
        // change this implementation to look like the one for write
        InodeProxy::new(self.fs_meta_data, self.inode_table, self.disk.clone(), self.inumber)
    }
}

// inode iter returns byte by byte
impl<'a> Iterator for InodeReadIter<'a> {
    type Item = u8;
    fn next(&mut self) -> Option<Self::Item> {
        if self.byte_offset as u32 == self.get_inode().size() {
            return None
        }

        let block_index = (self.byte_offset as f64 / Disk::BLOCK_SIZE as f64).floor() as usize;
        let block_offset = self.byte_offset % Disk::BLOCK_SIZE;

        if block_index == self.curr_block_index {
            self.byte_offset += 1;
            return Some(
                self.curr_data_block.data()[block_offset]
            )
        }
        
        if block_index < self.data_blocks.len() && self.data_blocks[block_index] > 0 {
            let mut block = Block::new();
            self.disk.read(self.data_blocks[block_index] as usize, block.data_as_mut());
            self.curr_data_block = block;
            self.curr_block_index += 1;
            self.byte_offset += 1;
            return Some(
                self.curr_data_block.data()[block_offset]
            )
        }
        None
    }
}