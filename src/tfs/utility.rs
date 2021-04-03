use super::disk::Disk;
use super::types::*;
use super::constants::*;

// ************ UTILITY FUNCTIONS ************************************
pub fn fetch_block(disk: &mut Disk, block_num: usize) -> Block {
    let mut block = Block::new();
    disk.read(block_num, block.data_as_mut());
    return block;
}

pub fn get_inode_block(inode_table: &Vec<(u32, InodeBlock)>, i: usize) -> (u32, Block) {
    let index = (i as f64 / (INODES_PER_BLOCK - 1) as f64).floor() as usize;
    let (block_num, inodeblock) = inode_table[index];
    // println!("GET INODE BLOCK: {}, {}", index, block_num);
    (block_num, inodeblock.as_block())
}

pub fn set_inode(inode_table: &mut Vec<(u32, InodeBlock)>, inumber: usize, inode: Inode) {
    let (blk_n, mut block) = get_inode_block(inode_table, inumber);
    let i = inumber % (INODES_PER_BLOCK - 1);
    block.iblock_as_mut().inodes[i] = inode;
    // println!("SET INODE ==> Inumber: {}, inode: {:?}", inumber, inode);
    let index = (inumber as f64 / (INODES_PER_BLOCK - 1) as f64).floor() as usize;
    inode_table[index] = (blk_n, block.inode_block());
}

pub fn as_u32_be(array: &[u8; 4]) -> u32 {
    ((array[0] as u32) << 24) +
    ((array[1] as u32) << 16) +
    ((array[2] as u32) <<  8) +
    ((array[3] as u32) <<  0)
}

pub fn as_u32_le(array: &[u8; 4]) -> u32 {
    ((array[0] as u32) <<  0) +
    ((array[1] as u32) <<  8) +
    ((array[2] as u32) << 16) +
    ((array[3] as u32) << 24)
}

pub fn as_u32(array: &[u8]) -> u32 {
    let mut byte_array: [u8; 4] = [0; 4];
    byte_array[0] = array[0];
    byte_array[1] = array[1];
    byte_array[2] = array[2];
    byte_array[3] = array[3];

    as_u32_be(&byte_array)
}

// ************ UTILITY FUNCTIONS ************************************
