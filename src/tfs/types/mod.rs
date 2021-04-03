
use super::disk::Disk;
use super::constants::*;

#[derive(Copy, Clone)]
#[allow(dead_code)]
pub struct MetaData {
    pub superblock: Superblock,
    pub inodes_root_block: Block,
    pub root_free_list: Block,
    pub inodes_free_list: Block
}

// the file system type
pub struct FileSystem<'a> {
    pub meta_data: Option<MetaData>,
    pub disk: Option<Disk<'a>>,
    pub inode_table: Option<Vec<(u32, InodeBlock)>>
}


pub mod inode;
pub mod i_list;
pub mod read_iter;
pub mod write_iter;
pub mod data_ptr;
pub mod block;

pub use inode::*;
pub use i_list::*;
pub use read_iter::*;
pub use write_iter::*;
pub use data_ptr::*;
pub use block::*;