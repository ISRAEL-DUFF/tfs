mod disk;
mod types;
mod utility;

use self::disk::Disk;
use self::types::*;

pub struct FileSystem;

impl FileSystem {
    fn debug(disk: &mut Disk) {
        let metaData = Self::read_meta_data(disk);
        let superblock = metaData.superBlock;
        let inodeTable = metaData.inodeTable;

        println!("********* SUPER BLOCK ***********");
        println!("Magic Number  {} is valid", superblock.MagicNumber);
        println!("{} blocks", superblock.Blocks);
        println!("{} inode blocks", superblock.InodeBlocks);
        println!("{} inodes", superblock.Inodes);
        println!("********* END SUPER ***********\n");


        let mut i_number = 0;
        let mut end_of_tab = false;
        println!("********* INODE INFO ***********");
        for inodes in inodeTable.iter() {
            if !end_of_tab {
                for inode in inodes.iter() {
                    if i_number < superblock.Inodes {
                        i_number += 1;
                        println!("Inode {}:", i_number);
                        println!("  size: {} bytes", inode.Size);
                        println!("  direct blocks: {}", inode.Direct.len())
                    } else {
                        end_of_tab = true;
                        break
                    }
                }
            }
        }
        println!("********* END INODE ***********\n");

    }

    fn format(disk: &mut Disk) -> bool {
        // STEP 1: set aside 10% of blocks for inodes
        let total_inode_blocks = (disk.size() as f64 * 0.1).ceil() as usize;

        // STEP 2: clear the inode table
        for i in 1..total_inode_blocks + 1 {
            disk.write(i, &mut [0; Disk::BLOCK_SIZE]);
        }

        // STEP 3: write the super block
        let superblock = Block {
            Super: Superblock {
                MagicNumber: MAGIC_NUMBER as u32,
                Blocks: disk.size() as u32,
                InodeBlocks: total_inode_blocks as u32,
                Inodes: 0
            }
        };
        disk.write(0, &mut superblock.data());

        true
    }

    // helper functions

    fn read_meta_data(disk: &mut Disk) -> MetaData {
        // read the super block
        let mut block = Block::new();
        let mut d = block.data();
        disk.read(0, &mut d);
        block.set_data(d);
        let superBlock = block.superblock();
        
        // read inode blocks ====> read the inode table
        let mut inodeTable = Vec::new();
        for i in 0..superBlock.InodeBlocks {
            disk.read(1 + i as usize, &mut block.data());
            inodeTable.push(block.inodes());
        }

        MetaData {
            superBlock,
            inodeTable
        }
    }
}

pub mod prelude {
    pub use super::disk::*;
    pub use super::types::*;
}


// tests
#[cfg(test)]
mod tests {
    use super::*;

    // #[test]
    fn test_debug() {
        let mut disk = Disk::new();
        disk.open("./data/image.5", 5);
        let mut data = [0; Disk::BLOCK_SIZE];
        disk.write(0,&mut data);

        assert_eq!(FileSystem::debug(&mut disk), ());
    }

    #[test]
    fn test_format() {
        let mut disk = Disk::new();
        disk.open("./data/image.20", 20);

        // let mut data = [0; Disk::BLOCK_SIZE];

        assert_eq!(FileSystem::format(&mut disk), true);
        // disk.read(0, &mut data);
        // println!("TEST READ: {}", data[2]);

        FileSystem::debug(&mut disk)
    }
}