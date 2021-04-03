// mod disk;
use super::disk::Disk;

pub const MAGIC_NUMBER: usize = 0xf0f03410;
pub const POINTERS_PER_INODE: usize = 5;
pub const INODE_SIZE: usize = 32; // the inode size in bytes
pub const INODES_PER_BLOCK: usize = Disk::BLOCK_SIZE / INODE_SIZE;
pub const POINTERS_PER_BLOCK: usize = 1024; // this is calculated as: disk.BLOCK_SIZE / 4

pub const INODES_ROOT_BLOCK: usize = 1;
pub const ROOT_FREE_LIST: usize = 2;
pub const INODES_FREE_LIST: usize = 3;
