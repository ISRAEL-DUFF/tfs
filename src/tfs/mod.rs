mod disk;
mod types;
mod utility;

use self::disk::Disk;
use self::types::*;

pub struct FileSystem<'a> {
    pub metaData: Option<MetaData>,
    pub inodeBitMap: Option<Vec<bool>>,
    pub dataBitMap: Option<Vec<bool>>,
    pub disk: Option<Disk<'a>>
}

impl<'a> FileSystem<'a> {
    fn new() -> Self {
        FileSystem {
            metaData: None,
            inodeBitMap: None,
            dataBitMap: None,
            disk: None
        }
    }

    fn from_disk(disk: &mut Disk<'a>) -> Self {
        let mut fs = Self::new();

        // STEP 1: try mounting the disk
        let mut disk_clone = disk.clone();
        let mut disk_clone2 = disk.clone();
        let mounted = fs.mount(disk);

        if !mounted {
            let formatted = Self::format(&mut disk_clone);
            if formatted {
                fs.mount(&mut disk_clone2);
            }
        }

        fs
    }

    fn info(&self) {
        match &self.metaData {
            Some(metaData) => Self::debug_print(metaData),
            _ => {}
        }
    }

    fn debug(disk: &mut Disk<'a>) {
        let metaData = Self::read_meta_data(disk);
        Self::debug_print(&metaData);
    }

    fn debug_print(meta_data: &MetaData) {
        let superblock = &meta_data.superBlock;
        let inodeTable = &meta_data.inodeTable;

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

    fn format(disk: &mut Disk<'a>) -> bool {
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

    fn mount(&mut self, disk: &mut Disk<'a>) -> bool {
        let metaData = Self::read_meta_data(disk);
        let metaData2 = Self::read_meta_data(disk); // ...have no idea how to make a copy
        // println!("metaData: {:?}", metaData.inodeTable);
        // println!("metaData2: {:?}", metaData.superBlock);

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
                            data_bit_map[(*direct_ptr - inodeBlocks - 1) as usize] = true
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
                                data_bit_map[(*ptr - inodeBlocks - 1) as usize] = true
                            }
                        }
                    }
                } else {
                    inode_bit_map.push(false);
                }
            }
        }

        // println!("inode BIT MAP: {:?}", inode_bit_map);
        // println!("data BIT MAP: {:?}", data_bit_map);

        self.inodeBitMap = Some(inode_bit_map);
        self.dataBitMap = Some(data_bit_map);
        self.disk = Some(disk.clone());
        true
    }

    fn create(&mut self) -> usize {
        // locate free inode in inode table
        match &mut self.inodeBitMap {
            Some(i_bitmap) => {
                let mut inumber = 0;
                for bit_map in i_bitmap.iter() {
                    if *bit_map == false {
                        i_bitmap[inumber] = true;
                        let mut inode = Inode::blank();
                        if self.load_inode(inumber, &mut inode) {
                            inode.Valid = 1;
                            self.save_inode(inumber, &mut inode);

                            match &mut self.metaData {
                                Some(metaData) => {
                                    metaData.superBlock.Inodes += 1;
                                    self.save_super_block();
                                },
                                _ => {}
                            }

                            return inumber
                        }
                        return 0
                    }
                    inumber += 1;
                }
                inumber
            },
            None => 0
        }
    }

    fn remove(&mut self, inumber: usize) -> bool {
        // load inode info
        let mut inode = Inode::blank();
        let inode_loaded = self.load_inode(inumber, &mut inode);

        let inode_loaded = match &mut self.metaData {
            Some(metaData) => {
                match &mut self.disk {
                    Some(disk) => {
                        if inode_loaded {
                            inode.Direct = [0; POINTERS_PER_INODE]; // free direct blocks
                            inode.Indirect = 0;   // free indirect blocks
                            inode.Valid = 0;     // set inode to invalid
                            true
                        } else {
                            false
                        }
                    },
                    _ => false
                }
            },
            None => false
        };

        if inode_loaded {
            // save inode
            self.save_inode(inumber, &mut inode);

            match &mut self.metaData {
                Some(metaData) => {
                    metaData.superBlock.Inodes -= 1;
                    self.save_super_block();
                },
                _ => {}
            }

            // clear inode in inode table
            match &mut self.inodeBitMap {
                Some(ibitMap)=> {
                    ibitMap[inumber] = false;
                    self.save_inode_table();
                },
                _ => {}
            };

            true
        } else {
            true
        }

    }

    fn stat(&mut self, inumber: usize) -> i64 {
        let mut inode = Inode::blank();
        if self.load_inode(inumber, &mut inode) {
            inode.Size as i64
        } else {
            -1
        }
    }

    fn read(&mut self, inumber: usize, data: &mut [u8], length: usize, offset: usize) -> i64 {
        // load inode info
        let mut inode = Inode::blank();
        if !self.load_inode(inumber, &mut inode) {
            return -1;
        }

        // adjust length
        let mut read_length = length;
        let total_length = offset + length;
        if total_length > inode.Size as usize {
            read_length = length - (total_length - inode.Size as usize);
        }

        // Read block and copy data
        match &mut self.disk {
            Some(disk) => {
                // let mut data_index = 0; // index into the 'data' parameter
                let mut blk_data = [0; Disk::BLOCK_SIZE];
                let mut bytes_read = 0;
                for blk in inode.Direct.iter() {
                    if *blk > 0 {
                        disk.read(*blk as usize, &mut blk_data);
                        for d in blk_data.iter() {
                            if bytes_read < read_length {
                                data[bytes_read] = *d;
                                // data_index += 1;
                                bytes_read += 1;
                            } else {
                                return bytes_read as i64;
                            }
                        }
                    } else {
                        return bytes_read as i64;
                    }
                }

                // still haven't got to end of file yet? --- read indirect block
                if bytes_read < read_length {
                    let mut indirect_block = Block::new();
                    let mut indirect_blk_data = indirect_block.data();
                    if inode.Indirect > 0 {
                        disk.read(inode.Indirect as usize, &mut indirect_blk_data);
                        indirect_block.set_data(indirect_blk_data);
                        let indirect_pointers = indirect_block.pointers();

                        for blk in indirect_pointers.iter() {
                            if *blk > 0 {
                                disk.read(*blk as usize, &mut blk_data);
                                for d in blk_data.iter() {
                                    if bytes_read < read_length {
                                        data[bytes_read] = *d;
                                        bytes_read += 1;
                                    } else {
                                        return bytes_read as i64;
                                    }
                                }
                            } else {
                                return bytes_read as i64;
                            }
                        }
                    }
                }
                return bytes_read as i64;
            },
            _ => {
                return -1;
            }
        }
    }

    fn write(&mut self, inumber: usize, data: &mut [u8], length: usize, offset: usize) -> i64 {
        // load inode
        let mut inode = Inode::blank();
        if !self.load_inode(inumber, &mut inode) {
            return -1;
        }

        // write block and copy to inode data

        // locate the last block in this inode
        let mut bytes_writen = 0;
        let block_index = (inode.Size as f64 / Disk::BLOCK_SIZE as f64).floor() as usize;
        let mut block_offset: usize = (inode.Size as usize % Disk::BLOCK_SIZE) as usize;
        // println!("COMPUTED BLOCK INDEX: {}", block_index);
        let mut block_num: usize;
        if block_index < POINTERS_PER_BLOCK {
            if inode.Direct[block_index] > 0 {
                block_num = inode.Direct[block_index] as usize;
            } else {
                // println!("ALLOCATING NEW DIRECT PTR BLOCK...");
                let blk = self.allocate_free_block(inumber);
                if blk == -1 {
                    return -1;
                }
                inode.Direct[block_index] = blk as u32;
                block_num = blk as usize;
            }
        } else {
            block_num = match &mut self.disk {
                Some(disk) => {
                    let mut block = Block::new();
                    let mut blk_d = block.data();
                    disk.read(inode.Indirect as usize, &mut blk_d);
                    block.set_data(blk_d);
                    let mut ptrs = block.pointers();

                    if ptrs[block_index - POINTERS_PER_BLOCK] > 0 {
                        ptrs[block_index - POINTERS_PER_BLOCK] as usize
                    } else {
                        // println!("ALLOCATING NEW INDIRECT PTR BLOCK...");
                        let blk = self.allocate_free_block(inumber);
                        if blk == -1 {
                            return -1;
                        }
                        ptrs[block_index - POINTERS_PER_BLOCK] = blk as u32;
                        blk as usize
                    }
                },
                _ => {
                    return -1;
                }
            }
        }

        println!("BLOCK NUMBER: {}", block_num);


        loop {
            if bytes_writen < length {
                let bytes_wr = self.write_to_block(block_num, block_offset, data, length - bytes_writen, offset + bytes_writen);
                println!("bytes_wr: {}", bytes_wr);
                if bytes_wr == 0 {
                    // allocate more blocks for data
                    let blk = self.allocate_free_block(inumber);
                    if blk == -1 {
                        return -1;
                    }
                    block_num = blk as usize;
                    block_offset = 0;
                    // println!("ALLOCATED BLOCK: {}", block_num);
                } else if bytes_wr == -1 {
                    return -1;  // something bad happened
                } else {
                    bytes_writen += bytes_wr as usize;
                    inode.Size += bytes_wr as u32;
                    block_offset = (inode.Size as usize % Disk::BLOCK_SIZE) as usize;
                    // println!(
                    //     "Writing data... block_num: {}, byte_writen: {}, bytes_wr: {}, file size: {}, length: {}", 
                    //     block_num, block_num, bytes_wr, inode.Size, length
                    // );
                }
            } else {
                self.save_inode(inumber, &mut inode);
                println!("Bytes writen: {}", bytes_writen);
                return bytes_writen as i64;
            }
        }

        bytes_writen as i64
    }

    // ****************** helper methods and functions *******************

    fn read_meta_data(disk: &mut Disk<'a>) -> MetaData {
        // read the super block
        let mut block = Block::new();
        let mut d = block.data();
        disk.read(0, &mut d);
        block.set_data(d);
        let superBlock = block.superblock();
        
        // read inode blocks ====> read the inode table
        let mut inodeTable = Vec::new();
        for i in 0..superBlock.InodeBlocks {
            let mut d = block.data();
            disk.read(1 + i as usize, &mut d);
            block.set_data(d);
            inodeTable.push(block.inodes());
        }

        MetaData {
            superBlock,
            inodeTable
        }
    }

    fn save_super_block(&mut self) -> bool {
        match &mut self.metaData {
            Some(metaData) => {
                match &mut self.disk {
                    Some(disk) => {
                        let mut block = Block::new();
                        block.set_superblock(metaData.superBlock);
                        disk.write(0, &mut block.data());

                        true
                    },
                    _ => false
                }
            },
            None => false
        }
    }

    fn save_inode_table(&mut self) -> bool {
        match &mut self.metaData {
            Some(metaData) => {
                match &mut self.disk {
                    Some(disk) => {
                        let mut block = Block::new();

                        // write in memory inodeTable to disk
                        let mut i = 1;
                        for inode_blk in metaData.inodeTable.iter() {
                            block.set_inodes(*inode_blk);
                            disk.write(i, &mut block.data());
                            i += 1;
                        }

                        true
                    },
                    _ => false
                }
            },
            None => false
        }
    }

    fn load_inode(&mut self, inumber: usize, inode: &mut Inode) -> bool {
        let mut blk = ((inumber / INODES_PER_BLOCK) as f64).ceil() as usize;
        if blk == 0 {
            blk = 1;
        }

        // read the block
        let mut block = Block::new();
        let mut data = block.data();
        match &mut self.disk {
            Some(disk) => {
                disk.read(blk, &mut data);
                block.set_data(data);
            },
            _ => {
                return false
            }
        };

        // interpret block as inodes and load into inode
        let inodes = block.inodes();
        let inode_r = inodes[inumber % INODES_PER_BLOCK];
        inode.Valid = inode_r.Valid;
        inode.Size = inode_r.Size;
        inode.Direct = inode_r.Direct;
        inode.Indirect = inode_r.Indirect;
        true
    }

    fn save_inode(&mut self, inumber: usize, inode: &mut Inode) -> bool {
        let mut blk = ((inumber / INODES_PER_BLOCK) as f64).ceil() as usize;

        if blk == 0 {
            blk = 1;
        }

        let row_blk = inumber % INODES_PER_BLOCK;

        // read the block
        let mut block = Block::new();
        let mut data = block.data();
        match &mut self.disk {
            Some(disk) => {
                disk.read(blk, &mut data);
                block.set_data(data);

                // interpret block as inodes and set inodes field
                let mut inodes = block.inodes();

                inodes[row_blk] = *inode;
                block.set_inodes(inodes);
                disk.write(blk, &mut block.data());

                // update in memory inodeTable
                match &mut self.metaData {
                    Some(metaData) => {
                        if metaData.inodeTable.len() < blk { // add inodes for block blk
                            println!("GGGGGGGGGGGGGG");
                            metaData.inodeTable.push(block.inodes());
                        } else {
                            metaData.inodeTable[blk - 1][row_blk] = *inode;
                        }
                    },
                    _ => {}
                };
            },
            _ => {
                return false
            }
        };

        true
    }

    fn allocate_free_block(&mut self, inumber: usize) -> i64 {
        // load inode
        let mut inode = Inode::blank();
        if !self.load_inode(inumber, &mut inode) {
            return -1;
        }

        let mut disk = match &mut self.disk {
            Some(disk) => {
                disk
            },
            _ => {
                return -1;
            }
        };

        let get_free_block = |dataBitMap: Option<&Vec<bool>>| {
            match dataBitMap {
                Some(dataBitMap) => {
                    // println!("DATABITMAP: {:?}", dataBitMap);
                    let mut i = 0;
                    loop {
                        if i < dataBitMap.len() {
                            if !dataBitMap[i] {
                                break (i + 1) as i64;
                            }
                        } else {
                            break -1;
                        }
                        i += 1;
                    }
                },
                _ => -1
            }
        };

    
        let free_block = get_free_block(self.dataBitMap.as_ref());

        if free_block > -1 {
            match &mut self.dataBitMap {
                Some(dataBitMap) => {
                    dataBitMap[free_block as usize - 1] = true;
                },
                _ => { return -1; }
            }

            match &mut self.metaData {
                Some(metaData) => {
                    let offset = metaData.superBlock.InodeBlocks;
                    let free_block = free_block + offset as i64;
                    // compute the inode number this free block belongs to
                    let mut i = 0;
                    loop {
                        if i == POINTERS_PER_INODE {
                            break;
                        }
                        if inode.Direct[i] == 0 {
                            inode.Direct[i] = free_block as u32;
                            self.save_inode(inumber, &mut inode);
                            return free_block;
                        }
                        i += 1;
                    }

                    if inode.Indirect == 0 {
                        // allocate new indirect block for more storage
                        let free_indirect = get_free_block(self.dataBitMap.as_ref());
                        if free_indirect < 0 {
                            return -1;
                        }
                        inode.Indirect = free_indirect as u32;
                        // self.save_inode(inumber, &mut inode);
                    }

                    let mut block = Block::new();
                    let mut block_data = block.data();
                    disk.read(inode.Indirect as usize, &mut block_data);
                    block.set_data(block_data);
                    let mut indirect_pointers = block.pointers();

                    let mut i = 0;
                    loop {
                        if i == POINTERS_PER_BLOCK {
                            break;
                        }

                        if indirect_pointers[i] == 0 {
                            indirect_pointers[i] = free_block as u32;
                            block.set_pointers(indirect_pointers);
                            disk.write(inode.Indirect as usize, &mut block.data());
                            self.save_inode(inumber, &mut inode);
                            return free_block;
                        }
                        i += 1;
                    }
                    return -1;
                },
                _ => {}
            }
        }

        -1
    }

    fn write_to_block(
        &mut self, block_num: usize, block_offset: usize, 
        data: &mut [u8], length: usize, offset: usize
    ) -> i64 {
        let mut bytes_writen = 0;
        if block_offset < Disk::BLOCK_SIZE && offset < data.len() { // this block is not filled yet
            let disc = match &mut self.disk {
                Some(disk) => disk,
                _ => {
                    return -1;
                }
            };
            let mut disk = disc.clone(); // just to avoid compiler error about multiple mutable ref of disc
            // println!("OVERFLOW: {}, {}, {}", offset + bytes_writen, length, data.len());

            let mut block_offset = block_offset;
            let mut length = length;
            if offset + length > data.len() {
                length = data.len() - offset;
            }
            let mut blk_data = [0; Disk::BLOCK_SIZE];
            disk.read(block_num, &mut blk_data);
            while block_offset < Disk::BLOCK_SIZE && bytes_writen < length {
                blk_data[block_offset] = data[offset + bytes_writen];
                bytes_writen += 1;
                block_offset += 1;
            }
            disk.write(block_num, &mut blk_data);
            return bytes_writen as i64;
        }
        return -1;
        // bytes_writen as i64
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

    // #[test]
    fn test_format() {
        let mut disk = Disk::new();
        disk.open("./data/image.20", 20);
        assert_eq!(FileSystem::format(&mut disk), true);
        FileSystem::debug(&mut disk);
    }

    // #[test]
    fn test_mount() {
        let mut disk = Disk::new();
        disk.open("./data/image.20", 20);
        let mut fs = FileSystem::new();
        assert_eq!(fs.mount(&mut disk), true);
    }

    // #[test]
    fn test_create_remove_inode() {
        let mut disk = Disk::from_file("./data/image.100", 100);
        let mut fs = FileSystem::from_disk(&mut disk);
        let inode1 = fs.create();
        let inode2 = fs.create();
        let inode3 = fs.create();
        let inode4 = fs.create();
        println!("created INODES: {}, {}, {}, {}", inode1, inode2, inode3, inode4);
        fs.info();

        fs.remove(inode2);
        println!("removed INODES: {}", inode2);

        fs.info()
    }

    fn to_mut_data (txt: &str) -> [u8; 4096] {
        let data = txt.as_bytes();
        let mut d = [0; 4096];
        let j = 0;
        for i in data.iter() {
            d[j] = *i;
        }
        d
    }

    #[test]
    fn test_fs_read_write() {
        let mut disk = Disk::from_file("./data/image.100", 100);
        let mut fs = FileSystem::from_disk(&mut disk);
        let inode1 = fs.create();
        let mut data = to_mut_data("Hello, World this is great string");
        let mut data_r = [0; 4096];
        let mut i = 0;
        // loop {
        //     if i <= 10 {
        //         fs.write(inode1, &mut data, (inode1 +1 + i) * 450, i);
        //         i += 1;
        //     } else {
        //         break;
        //     }
        // }
        fs.write(inode1, &mut data, 33, 1);
        FileSystem::debug(&mut disk);
        fs.info();

        fs.read(inode1, &mut data_r, 33, 1);

        assert_eq!(data_r, data);

        let strrr1 = std::str::from_utf8(&data_r).unwrap().to_string();
        let strrr2 = std::str::from_utf8(&data).unwrap().to_string();

        println!("STRING 1: {}, STRING 2: {}", strrr1, strrr2);

    }
}