mod disk;
mod types;
mod utility;

use self::disk::Disk;
use self::types::*;

pub struct FileSystem<'a> {
    pub meta_data: Option<MetaData>,
    pub inode_bitmap: Option<Vec<bool>>,
    pub data_bitmap: Option<Vec<bool>>,
    pub disk: Option<Disk<'a>>
}

impl<'a> FileSystem<'a> {
    pub fn new() -> Self {
        FileSystem {
            meta_data: None,
            inode_bitmap: None,
            data_bitmap: None,
            disk: None
        }
    }

    // pub fn clone(&self) -> Self {
    //     FileSystem {
    //         metaData: self.metaData,
    //         inodeBitMap: self.inodeBitMap,
    //         dataBitMap: self.dataBitMap,
    //         disk: self.disk
    //     }
    // }

    pub fn from_disk(disk: &mut Disk<'a>) -> Self {
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

    pub fn info(&self) {
        match &self.meta_data {
            Some(meta_data) => Self::debug_print(meta_data),
            _ => {}
        }
    }

    pub fn debug(disk: &mut Disk<'a>) {
        let meta_data = Self::read_meta_data(disk);
        Self::debug_print(&meta_data);
    }

    pub fn debug_print(meta_data: &MetaData) {
        let superblock = &meta_data.superblock;
        let inode_table = &meta_data.inode_table;

        println!("********* SUPER BLOCK ***********");
        println!("Magic Number  {} is valid", superblock.magic_number);
        println!("{} blocks", superblock.blocks);
        println!("{} inode blocks", superblock.inode_blocks);
        println!("{} inodes", superblock.inodes);
        println!("********* END SUPER ***********\n");


        let mut i_number = 0;
        let mut end_of_tab = false;
        println!("********* INODE INFO ***********");
        for inodes in inode_table.iter() {
            if !end_of_tab {
                for inode in inodes.iter() {
                    if i_number < superblock.inodes {
                        i_number += 1;
                        println!("Inode {}:", i_number);
                        println!("  size: {} bytes", inode.size);
                        println!("  direct blocks: {}", inode.direct.len())
                    } else {
                        end_of_tab = true;
                        break
                    }
                }
            }
        }
        println!("********* END INODE ***********\n");

    }

    pub fn format(disk: &mut Disk<'a>) -> bool {
        // STEP 1: set aside 10% of blocks for inodes
        let total_inode_blocks = (disk.size() as f64 * 0.1).ceil() as usize;

        // STEP 2: clear the inode table
        for i in 1..total_inode_blocks + 1 {
            disk.write(i, &mut [0; Disk::BLOCK_SIZE]);
        }

        // STEP 3: write the super block
        let superblock = Block {
            superblock: Superblock {
                magic_number: MAGIC_NUMBER as u32,
                blocks: disk.size() as u32,
                inode_blocks: total_inode_blocks as u32,
                inodes: 0
            }
        };
        disk.write(0, &mut superblock.data());

        true
    }

    pub fn mount(&mut self, disk: &mut Disk<'a>) -> bool {
        let meta_data = Self::read_meta_data(disk);
        let meta_data2 = Self::read_meta_data(disk); // ...have no idea how to make a copy
        // println!("metaData: {:?}", metaData.inodeTable);
        // println!("metaData2: {:?}", metaData.superBlock);

        if meta_data.superblock.magic_number != MAGIC_NUMBER as u32 {
            return false
        }

        self.meta_data = Some(meta_data2);

        let nBlocks = meta_data.superblock.blocks;
        let inodeBlocks = meta_data.superblock.inode_blocks;

        let mut inode_bit_map = Vec::new();
        let mut data_bit_map = Vec::new();

        // fill the data bit map to unused by default
        for _i in 0..nBlocks - inodeBlocks - 1 {
            data_bit_map.push(false);
        }

        for inodes in meta_data.inode_table.iter() {
            for inode in inodes.iter() {
                if inode.valid == 1u32 {
                    inode_bit_map.push(true);

                    // next, follow the direct blocks to see what data blocks it has
                    for direct_ptr in inode.direct.iter() {
                        if *direct_ptr != 0u32 {
                            data_bit_map[(*direct_ptr - inodeBlocks - 1) as usize] = true
                        }
                    }

                    // also, does this inode has indirect block?
                    if inode.single_indirect != 0u32 { // if so, read the block and scan
                        let mut tmp_block = Block::new();
                        let mut tmp_data = tmp_block.data();
                        disk.read(inode.single_indirect as usize, &mut tmp_data);
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

        self.inode_bitmap = Some(inode_bit_map);
        self.data_bitmap = Some(data_bit_map);
        self.disk = Some(disk.clone());
        true
    }

    pub fn create(&mut self) -> usize {
        // locate free inode in inode table
        match &mut self.inode_bitmap {
            Some(i_bitmap) => {
                let mut inumber = 0;
                for bit_map in i_bitmap.iter() {
                    if *bit_map == false {
                        i_bitmap[inumber] = true;
                        let mut inode = Inode::blank();
                        if self.load_inode(inumber, &mut inode) {
                            inode.valid = 1;
                            self.save_inode(inumber, &mut inode);

                            match &mut self.meta_data {
                                Some(meta_data) => {
                                    meta_data.superblock.inodes += 1;
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

    pub fn remove(&mut self, inumber: usize) -> bool {
        // load inode info
        let mut inode = Inode::blank();
        let inode_loaded = self.load_inode(inumber, &mut inode);

        let inode_loaded = match &mut self.meta_data {
            Some(_meta_data) => {
                match &mut self.disk {
                    Some(_disk) => {
                        if inode_loaded {
                            inode.direct = [0; POINTERS_PER_INODE]; // free direct blocks
                            inode.single_indirect = 0;   // free indirect blocks
                            inode.valid = 0;     // set inode to invalid
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

            match &mut self.meta_data {
                Some(meta_data) => {
                    meta_data.superblock.inodes -= 1;
                    self.save_super_block();
                },
                _ => {}
            }

            // clear inode in inode table
            match &mut self.inode_bitmap {
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

   pub fn stat(&mut self, inumber: usize) -> i64 {
        let mut inode = Inode::blank();
        if self.load_inode(inumber, &mut inode) {
            inode.size as i64
        } else {
            -1
        }
    }

    fn read_from_block(
        &mut self, block_num: usize, data_offset: usize, 
        data: &mut [u8], length: usize, offset: usize
    ) -> i64 {
        // adjust length
        let mut read_length = length;
        let total_length = data_offset + length;
        if total_length > Disk::BLOCK_SIZE {
            read_length = length - (total_length - Disk::BLOCK_SIZE);

            if data_offset + read_length > data.len() {
                read_length = data.len() - data_offset;
            }
        }

        // sanity check
        if offset > Disk::BLOCK_SIZE || data_offset > data.len(){
            return -1;
        }

        // read data from disk
        match &mut self.disk {
            Some(disk) => {
                let mut blk_data = [0; Disk::BLOCK_SIZE];
                disk.read(block_num, &mut blk_data);
                let mut bytes_read = 0;
                let mut i = offset;
                while i < Disk::BLOCK_SIZE {
                    if bytes_read < read_length {
                        data[data_offset + bytes_read] = blk_data[i];
                        bytes_read += 1;
                        i += 1;
                    } else {
                        return bytes_read as i64;
                    }
                }
                return bytes_read as i64;
            },
            _ => {
                return -1;
            }
        }
    }

    pub fn read(
        &mut self, inumber: usize, 
        data: &mut [u8], length: usize, offset: usize
    ) -> i64 {
        // load inode info
        let mut inode = Inode::blank();
        if !self.load_inode(inumber, &mut inode) {
            return -1;
        }

        // adjust length
        let mut length = length;
        let total_length = offset + length;
        if total_length > inode.size as usize {
            length = length - (total_length - inode.size as usize);
        }

        // compute initial block and offset index
        let block_index = (offset as f64 / Disk::BLOCK_SIZE as f64).floor() as usize;
        let block_offset = offset % Disk::BLOCK_SIZE;

        let mut block = Block::new();
        let mut has_indirect_blk = false;
        let mut indirect_blks: [u32; POINTERS_PER_BLOCK] = [0; POINTERS_PER_BLOCK];
        let mut bytes_read = 0;
        let mut j = 0;
        loop {
            if bytes_read == length || j == length {
                return bytes_read as i64;
            }

            if block_index < POINTERS_PER_INODE {
                if inode.direct[block_index] > 0 {
                    // use the block number for reading
                    let r = self.read_from_block(
                        inode.direct[block_index] as usize, bytes_read as usize, 
                        data, length - bytes_read, block_offset
                    );
                    if r > -1 {
                        bytes_read += r as usize;
                    } else {
                        return -1;
                    }
                } else {
                    return -1;
                }
            } else {
                if !has_indirect_blk {
                    match &mut self.disk {
                        Some(disk) => {
                            let mut blk_d = block.data();
                            disk.read(inode.single_indirect as usize, &mut blk_d);
                            block.set_data(blk_d);
                            indirect_blks = block.pointers();
                            has_indirect_blk = true;
                        },
                        _ => { return -1; }
                    }
                }
                
                // use block index to get block number from indirect pointer
                let indirect_blk_index = block_index - POINTERS_PER_INODE;
                if indirect_blks[indirect_blk_index] > 0 {
                    let r = self.read_from_block(
                        indirect_blks[indirect_blk_index] as usize, bytes_read as usize, 
                        data, length - bytes_read, block_offset
                    );
                    if r > -1 {
                        bytes_read += r as usize;
                    } else {
                        return -1;
                    }
                } else {
                    return -1;
                }
            }
            j += 1;
            println!("BYTES INDEX: {}, {}", block_index, block_offset);
        }        
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
        return bytes_writen as i64;
    }

    pub fn write(&mut self, inumber: usize, data: &mut [u8], length: usize, offset: usize) -> i64 {
        // load inode
        let mut inode = Inode::blank();
        if !self.load_inode(inumber, &mut inode) {
            return -1;
        }

        let disc = match &mut self.disk {
            Some(d) => d,
            _ => { return -1; }
         };
         let mut disk = disc.clone();

        // locate the last block in this inode
        let mut bytes_writen = 0;
        let block_index = (inode.size as f64 / Disk::BLOCK_SIZE as f64).floor() as usize;
        let mut block_offset: usize = (inode.size as usize % Disk::BLOCK_SIZE) as usize;
        let mut block_num: usize;
        if block_index < POINTERS_PER_INODE {
            if inode.direct[block_index] > 0 {
                block_num = inode.direct[block_index] as usize;
            } else {
                let blk = self.allocate_free_block(inumber);
                if blk == -1 {
                    return -1;
                }
                inode.direct[block_index] = blk as u32;
                block_num = blk as usize;
            }
        } else {
            // make sure an indirect block has been allocated
            if inode.single_indirect == 0 {
                let blk = self.allocate_free_block(inumber);
                if blk == -1 {
                    return -1;
                }
                inode.single_indirect = blk as u32;
                self.save_inode(inumber, &mut inode);
                disk.write(blk as usize, &mut [0; Disk::BLOCK_SIZE]);
            }

            let mut block = Block::new();
            let mut blk_d = block.data();


            disk.read(inode.single_indirect as usize, &mut blk_d);
            block.set_data(blk_d);
            let mut ptrs = block.pointers();

            if ptrs[block_index - POINTERS_PER_INODE] > 0 {
               block_num = ptrs[block_index - POINTERS_PER_INODE] as usize;
            } else {
                let blk = self.allocate_free_block(inumber);
                if blk == -1 {
                    return -1;
                }
                ptrs[block_index - POINTERS_PER_INODE] = blk as u32;
                block_num = blk as usize
            }
        }

        // write block and copy to inode data
        loop {
            if bytes_writen < length && (offset + bytes_writen) < data.len() {
                let bytes_wr = self.write_to_block(block_num, block_offset, data, length - bytes_writen, offset + bytes_writen);
                println!("bytes_writen: {}", bytes_wr);
                if bytes_wr == 0 {
                    // allocate more blocks for data
                    let blk = self.allocate_free_block(inumber);
                    if blk == -1 {
                        return -1;
                    }
                    block_num = blk as usize;
                    block_offset = 0;
                } else if bytes_wr == -1 {
                    return -1;  // something bad happened
                } else {
                    bytes_writen += bytes_wr as usize;
                    inode.size += bytes_wr as u32;
                    block_offset = (inode.size as usize % Disk::BLOCK_SIZE) as usize;
                }
            } else {
                self.save_inode(inumber, &mut inode);
                println!("Total bytes writen: {}", bytes_writen);
                return bytes_writen as i64;
            }
        }
    }

    // ****************** helper methods and functions *******************

    fn read_meta_data(disk: &mut Disk<'a>) -> MetaData {
        // read the super block
        let mut block = Block::new();
        let mut d = block.data();
        disk.read(0, &mut d);
        block.set_data(d);
        let superblock = block.superblock();
        
        // read inode blocks ====> read the inode table
        let mut inode_table = Vec::new();
        for i in 0..superblock.inode_blocks {
            let mut d = block.data();
            disk.read(1 + i as usize, &mut d);
            block.set_data(d);
            inode_table.push(block.inodes());
        }

        MetaData {
            superblock,
            inode_table
        }
    }

    fn save_super_block(&mut self) -> bool {
        match &mut self.meta_data {
            Some(meta_data) => {
                match &mut self.disk {
                    Some(disk) => {
                        let mut block = Block::new();
                        block.set_superblock(meta_data.superblock);
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
        match &mut self.meta_data {
            Some(meta_data) => {
                match &mut self.disk {
                    Some(disk) => {
                        let mut block = Block::new();

                        // write in memory inodeTable to disk
                        let mut i = 1;
                        for inode_blk in meta_data.inode_table.iter() {
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
        inode.valid = inode_r.valid;
        inode.size = inode_r.size;
        inode.direct = inode_r.direct;
        inode.single_indirect = inode_r.single_indirect;
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
                match &mut self.meta_data {
                    Some(meta_data) => {
                        if meta_data.inode_table.len() < blk { // add inodes for block blk
                            println!(" I don't think this section will ever execute");
                            meta_data.inode_table.push(block.inodes());
                        } else {
                            meta_data.inode_table[blk - 1][row_blk] = *inode;
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

        let disk = match &mut self.disk {
            Some(disk) => {
                disk
            },
            _ => {
                return -1;
            }
        };

        let get_free_block = |data_bitmap: Option<&Vec<bool>>| {
            match data_bitmap {
                Some(data_bitmap) => {
                    // println!("DATABITMAP: {:?}", dataBitMap);
                    let mut i = 0;
                    loop {
                        if i < data_bitmap.len() {
                            if !data_bitmap[i] {
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

    
        let free_block = get_free_block(self.data_bitmap.as_ref());

        if free_block > -1 {
            match &mut self.data_bitmap {
                Some(data_bitmap) => {
                    data_bitmap[free_block as usize - 1] = true;
                },
                _ => { return -1; }
            }

            match &mut self.meta_data {
                Some(meta_data) => {
                    let offset = meta_data.superblock.inode_blocks;
                    let free_block = free_block + offset as i64;
                    // compute the inode number this free block belongs to
                    let mut i = 0;
                    loop {
                        if i == POINTERS_PER_INODE {
                            break;
                        }
                        if inode.direct[i] == 0 {
                            inode.direct[i] = free_block as u32;
                            self.save_inode(inumber, &mut inode);
                            return free_block;
                        }
                        i += 1;
                    }

                    if inode.single_indirect == 0 {
                        // allocate new indirect block for more storage
                        let free_indirect = get_free_block(self.data_bitmap.as_ref());
                        if free_indirect < 0 {
                            return -1;
                        }
                        inode.single_indirect = free_indirect as u32;
                        // self.save_inode(inumber, &mut inode);
                    }

                    let mut block = Block::new();
                    let mut block_data = block.data();
                    disk.read(inode.single_indirect as usize, &mut block_data);
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
                            disk.write(inode.single_indirect as usize, &mut block.data());
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
        fs.info();

        fs.remove(inode2);

        fs.info()
    }

    fn to_mut_data (txt: &str) -> [u8; 4096] {
        let data = txt.as_bytes();
        let mut d = [0; 4096];
        let mut j = 0;
        for i in data.iter() {
            d[j] = *i;
            j += 1;
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
        
        let b1 = fs.write(inode1, &mut data, 33, 0);
        FileSystem::debug(&mut disk);
        fs.info();

        let b2 = fs.read(inode1, &mut data_r, 33, 0);

        let str1 = std::str::from_utf8(&data_r).unwrap().to_string();
        let str2 = std::str::from_utf8(&data).unwrap().to_string();

        println!("STRING 1: {}, STRING 2: {}", str1, str2);
        println!("bytes writen: {}, bytes read: {}", b1, b2);
    
        assert_eq!(str1, str2);

    }
}