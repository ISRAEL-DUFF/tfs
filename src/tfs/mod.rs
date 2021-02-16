mod disk;
mod types;
mod utility;

use self::disk::Disk;
use self::types::*;

pub struct FileSystem {
    pub metaData: Option<MetaData>,
    pub inodeBitMap: Option<Vec<bool>>,
    pub dataBitMap: Option<Vec<bool>>
}

impl FileSystem {
    fn new() -> Self {
        FileSystem {
            metaData: None,
            inodeBitMap: None,
            dataBitMap: None
        }
    }

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

    fn mount(&mut self, disk: &mut Disk) -> bool {
        let metaData = Self::read_meta_data(disk);
        let metaData2 = Self::read_meta_data(disk); // ...have no idea how to make a copy
        if metaData.superBlock.MagicNumber != MAGIC_NUMBER as u32 {
            return false
        }

        self.metaData = Some(metaData2);

        let nBlocks = metaData.superBlock.Blocks;
        let inodeBlocks = metaData.superBlock.InodeBlocks;

        let mut inode_bit_map = Vec::new();
        let mut data_bit_map = Vec::new();

        // fill the data bit map to unused by default
        for i in 0..nBlocks - inodeBlocks - 1 {
            data_bit_map.push(false);
        }

        for inodes in metaData.inodeTable.iter() {
            for inode in inodes.iter() {
                if inode.Valid == 1u32 {
                    inode_bit_map.push(true);

                    // next, follow the direct blocks to see what data blocks it has
                    for direct_ptr in inode.Direct.iter() {
                        if *direct_ptr != 0u32 {
                            data_bit_map[*direct_ptr as usize] = true
                        }
                    }

                    // also, does this inode has indirect block?
                    if inode.Indirect != 0u32 { // if so, read the block and scan
                        let mut tmp_block = Block::new();
                        let mut tmp_data = tmp_block.data();
                        disk.read(inode.Indirect as usize, &mut tmp_data);
                        tmp_block.set_data(tmp_data);

                        // interpret tmp_block as pointers, then scan
                        let indirect_ptr_blocks = tmp_block.pointers();
                        for ptr in indirect_ptr_blocks.iter() {
                            if *ptr != 0u32 {
                                data_bit_map[*ptr as usize] = true
                            }
                        }
                    }
                } else {
                    inode_bit_map.push(false);
                }
            }
        }

        self.inodeBitMap = Some(inode_bit_map);
        self.dataBitMap = Some(data_bit_map);

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
        assert_eq!(FileSystem::format(&mut disk), true);
        FileSystem::debug(&mut disk)
    }

    #[test]
    fn test_mount() {
        let mut disk = Disk::new();
        disk.open("./data/image.20", 20);
        let mut fs = FileSystem::new();
        assert_eq!(fs.mount(&mut disk), true);
    }
}