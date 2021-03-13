mod disk;
mod types;
mod utility;

use self::disk::Disk;
use self::types::*;

pub struct FileSystem<'a> {
    pub meta_data: Option<MetaData>,
    pub disk: Option<Disk<'a>>,
    pub inode_table: Option<Vec<(u32, InodeBlock)>>
}

impl<'a> FileSystem<'a> {
    pub fn new() -> Self {
        FileSystem {
            meta_data: None,
            disk: None,
            inode_table: None
        }
    }

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

    pub fn info(&mut self) {
        match &mut self.meta_data {
            Some(meta_data) => {
                match &mut self.disk {
                    Some(disk) => {
                        Self::debug_print(meta_data, disk)
                    },
                    None => {}
                }
            },
            _ => {}
        }
    }

    pub fn debug(disk: &mut Disk<'a>) {
        let mut meta_data = Self::read_meta_data(disk);
        Self::debug_print(&mut meta_data, disk);
    }

    pub fn debug_print(meta_data: &mut MetaData, disk: &mut Disk<'a>) {
        let superblock = &meta_data.superblock;

        println!("********* SUPER BLOCK ***********");
        println!("Magic Number  {} is valid", superblock.magic_number);
        println!("{} blocks", superblock.blocks);
        println!("Next free block position: {}", superblock.current_block_index);
        // println!("{} inode blocks", superblock.inode_blocks);
        println!("{} inodes", superblock.inodes);
        println!("********* END SUPER ***********\n");


        let mut i_number = 0;
        let mut end_of_tab = false;
        println!("********* INODE INFO ***********");
        // let inode_block = meta_data.inodes_root_block.inode_block();
        let mut inode_table = Self::generate_inode_table(meta_data, disk.clone());
        let mut inode_list = InodeList::new(meta_data, &mut inode_table, disk.clone());
        let mut i: usize = 0;
        // loop {
        //     if i < (meta_data.superblock.inodes + 2) as usize {
        //         // inode in inode_block.inodes.iter();
        //         println!("Inode {:?}:", inode_block.inodes[i]);
        //         println!("  size: {} bytes", inode_block.inodes[i].size);
        //         i += 1;
        //     } else {
        //         break;
        //     }
        // }
        for inode in inode_list.iter() {
            println!("Inode {:?}:", inode);
            // println!("  size: {} bytes", inode.size);
        }
        println!("********* END INODE ***********\n");

    }

    pub fn format(disk: &mut Disk<'a>) -> bool {   
        // STEP 1: write the super block
        let mut superblock = Block {
            superblock: Superblock {
                magic_number: MAGIC_NUMBER as u32,
                blocks: disk.size() as u32,
                inode_blocks: 0,
                current_block_index: 4,
                free_lists: 0,
                inodes: 0
            }
        };
        disk.write(0, superblock.data_as_mut());

        let mut block = Block::new();
        disk.write(1, block.data_as_mut());
        disk.write(2, block.data_as_mut());
        disk.write(3, block.data_as_mut());

        true
    }

    pub fn mount(&mut self, disk: &mut Disk<'a>) -> bool {
        let mut meta_data = Self::read_meta_data(disk);

        if meta_data.superblock.magic_number != MAGIC_NUMBER as u32 {
            return false
        }

        let disc = disk.clone();

        // generate the inode table
        let inode_table = Self::generate_inode_table(&mut meta_data, disk.clone());

        self.meta_data = Some(meta_data);
        self.disk = Some(disc);
        self.inode_table = Some(inode_table);

        // let inode_list = InodeList::new(self.meta_data.as_mut(), disk.clone());
        true
    }

    pub fn create(&mut self) -> i64 {
        let fs_raw_ptr = self as *mut Self;

        let meta_dat =  unsafe {
            match &mut (*fs_raw_ptr).meta_data {
                Some(meta_data) => meta_data,
                _ => {
                    return -1;
                }
            }
        };

        let disc = match &self.disk {
            Some(disk) => disk,
            _ => {
                return -1;
            }
        };

        let mut inode_table = unsafe {
            match &mut (*fs_raw_ptr).inode_table {
                Some(itable) => itable,
                _ => {
                    return -1;
                }
            }
        };

        let mut disk = disc.clone();


        // create the inode
        let inode = Inode::blank();
        let mut inode_list = InodeList::new(meta_dat, &mut inode_table, disk);

        
        if inode_list.inodeblock_isfull() {
            let new_blk = self.allocate_free_block(0);
            if !inode_list.add_inodeblock(new_blk) {
                return -1;
            }
        }
        
        
        let inumber = inode_list.add(inode);
        self.save_meta_data("all");
        inumber
    }

    pub fn remove(&mut self, inumber: usize) -> bool {
        // load inode info
        let mut inode = Inode::blank();
        let inode_loaded = self.load_inode(inumber, &mut inode);

        true
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
        // let mut inode = Inode::blank();
        // if !self.load_inode(inumber, &mut inode) {
        //     return -1;
        // }

        let fs_raw_ptr = self as *mut Self;

        let meta_dat =  unsafe {
            match &mut (*fs_raw_ptr).meta_data {
                Some(meta_data) => meta_data,
                _ => {
                    return -1;
                }
            }
        };

        let disc = match &self.disk {
            Some(disk) => disk,
            _ => {
                return -1;
            }
        };

        let mut inode_table = unsafe {
            match &mut (*fs_raw_ptr).inode_table {
                Some(itable) => itable,
                _ => {
                    return -1;
                }
            }
        };

        let mut disk = disc.clone();

        let mut inode_list = InodeList::new(meta_dat, &mut inode_table, disk.clone());
        // let mut inode_list2 = InodeList::new(meta_dat, &mut inode_table, disk);
        // let inode_iter = inode_list.read_iter(inumber);

        let mut i_list = &mut inode_list as *mut InodeList;

        let inode_iter = unsafe{(*i_list).read_iter(inumber)};

        // adjust length
        let mut length = length;
        let total_length = offset + length;
        let file_size = unsafe {(*i_list).get_inode(inumber).size()};
        if total_length > file_size as usize {
            length = length - (total_length - file_size as usize);
        }

        
        let mut i = 0;
        for byte in inode_iter {
            data[i] = byte;
            i += 1;
        }
        return i as i64


        // let (block_n, block_offset) = self.get_block_num(inumber, offset);
        // if block_n < 0 {
        //     return -1;
        // }

        // let mut bytes_read = 0;
        // let mut j = 0;
        // loop {
        //     if bytes_read == length || j == length {
        //         return bytes_read as i64;
        //     }

        //     // use the block number for reading
        //     let r = self.read_from_block(
        //         block_n as usize, bytes_read as usize, 
        //         data, length - bytes_read, block_offset
        //     );
        //     if r > -1 {
        //         bytes_read += r as usize;
        //     } else {
        //         return -1;
        //     }
 
        //     j += 1;
        //     println!("BYTES NUM: {}, {}", block_n, block_offset);
        // }        
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

    // fn this(&mut self) -> Self {
    //     let fs_raw_ptr = self as *mut Self;
    //    unsafe {
    //        (*fs_raw_ptr)
    //    }
    // }

    pub fn write(&mut self, inumber: usize, data: &mut [u8], length: usize, offset: usize) -> i64 {
        // // load inode
        // let mut inode = Inode::blank();
        // if !self.load_inode(inumber, &mut inode) {
        //     return -1;
        // }


        let fs_raw_ptr = self as *mut Self;

        let meta_dat =  unsafe {
            match &mut (*fs_raw_ptr).meta_data {
                Some(meta_data) => meta_data,
                _ => {
                    return -1;
                }
            }
        };

        let disc = match &self.disk {
            Some(disk) => disk,
            _ => {
                return -1;
            }
        };

        let mut inode_table = unsafe {
            match &mut (*fs_raw_ptr).inode_table {
                Some(itable) => itable,
                _ => {
                    return -1;
                }
            }
        };

        let mut inode_list = InodeList::new(meta_dat, &mut inode_table, disc.clone());
        let mut writer_iter = inode_list.write_iter(inumber);
        if !writer_iter.has_datablock() {
            unsafe {
                let data_blk = (*fs_raw_ptr).allocate_free_block(0);
                writer_iter.set_data_block(data_blk);
            }
        }
        let mut i = 0;
        loop {
            if i + offset < length {
                let r = writer_iter.write_byte(data[offset + i]);
                if r.1 < 0 {
                    unsafe {
                        let data_blk = (*fs_raw_ptr).allocate_free_block(0);
                        writer_iter.add_data_blk(data_blk);
                        continue;
                    }
                }
                // println!("RESUTL: {:?}", r);
            } else {
                break
            }
            i += 1;
        }
        writer_iter.flush();


        // let disc = match &mut self.disk {
        //     Some(d) => d,
        //     _ => { return -1; }
        //  };
        //  let mut disk = disc.clone();

        // // locate the last block in this inode
        // let mut bytes_writen = 0;
        // let block_index = (inode.size as f64 / Disk::BLOCK_SIZE as f64).floor() as usize;
        // let mut block_offset: usize = (inode.size as usize % Disk::BLOCK_SIZE) as usize;
        // let mut block_num: usize;
        // if block_index < POINTERS_PER_INODE {
        //     if inode.direct[block_index] > 0 {
        //         block_num = inode.direct[block_index] as usize;
        //     } else {
        //         let blk = self.allocate_free_block(inumber);
        //         if blk == -1 {
        //             return -1;
        //         }
        //         inode.direct[block_index] = blk as u32;
        //         block_num = blk as usize;
        //     }
        // } else {
        //     // make sure an indirect block has been allocated
        //     if inode.single_indirect == 0 {
        //         let blk = self.allocate_free_block(inumber);
        //         if blk == -1 {
        //             return -1;
        //         }
        //         inode.single_indirect = blk as u32;
        //         self.save_inode(inumber, &mut inode);
        //         disk.write(blk as usize, &mut [0; Disk::BLOCK_SIZE]);
        //     }

        //     let mut block = Block::new();
        //     let mut blk_d = block.data();


        //     disk.read(inode.single_indirect as usize, &mut blk_d);
        //     block.set_data(blk_d);
        //     let mut ptrs = block.pointers();

        //     if ptrs[block_index - POINTERS_PER_INODE] > 0 {
        //        block_num = ptrs[block_index - POINTERS_PER_INODE] as usize;
        //     } else {
        //         let blk = self.allocate_free_block(inumber);
        //         if blk == -1 {
        //             return -1;
        //         }
        //         ptrs[block_index - POINTERS_PER_INODE] = blk as u32;
        //         block_num = blk as usize
        //     }
        // }

        // // write block and copy to inode data
        // loop {
        //     if bytes_writen < length && (offset + bytes_writen) < data.len() {
        //         let bytes_wr = self.write_to_block(block_num, block_offset, data, length - bytes_writen, offset + bytes_writen);
        //         println!("bytes_writen: {}", bytes_wr);
        //         if bytes_wr == 0 {
        //             // allocate more blocks for data
        //             let blk = self.allocate_free_block(inumber);
        //             if blk == -1 {
        //                 return -1;
        //             }
        //             block_num = blk as usize;
        //             block_offset = 0;
        //         } else if bytes_wr == -1 {
        //             return -1;  // something bad happened
        //         } else {
        //             bytes_writen += bytes_wr as usize;
        //             inode.size += bytes_wr as u32;
        //             block_offset = (inode.size as usize % Disk::BLOCK_SIZE) as usize;
        //         }
        //     } else {
        //         self.save_inode(inumber, &mut inode);
        //         println!("Total bytes writen: {}", bytes_writen);
        //         return bytes_writen as i64;
        //     }
        // }
        -1
    }

    // ****************** helper methods and functions *******************

    fn read_meta_data(disk: &mut Disk<'a>) -> MetaData {
        // read the super block
        let mut block = fetch_block(disk, 0);
        let superblock = block.superblock();
        
        // read inode blocks ====> read the inode table
        let irb = fetch_block(disk, INODES_ROOT_BLOCK);
        let rfl = fetch_block(disk, ROOT_FREE_LIST);
        let ifl = fetch_block(disk, INODES_FREE_LIST);

        MetaData {
            superblock,
            inodes_root_block: irb,
            root_free_list: rfl,
            inodes_free_list: ifl
        }
    }

    fn save_meta_data(&mut self, name: &str) -> bool {
        match &mut self.meta_data {
            Some(meta_data) => {
                match &mut self.disk {
                    Some(disk) => {
                        let mut block = Block::new();
                        match name {
                            "superblock" => {
                                block.set_superblock(meta_data.superblock);
                                disk.write(0, block.data_as_mut());
                                true
                            },
                            "irb" => {
                                disk.write(INODES_ROOT_BLOCK, meta_data.inodes_root_block.data_as_mut());
                                true
                            },

                            "rfl" => {
                                disk.write(ROOT_FREE_LIST, meta_data.root_free_list.data_as_mut());
                                true
                            },
                            "ifl" => {
                                disk.write(INODES_FREE_LIST, meta_data.inodes_free_list.data_as_mut());
                                true
                            },
                            "all" => {
                                block.set_superblock(meta_data.superblock);
                                disk.write(0, block.data_as_mut());

                                //disk.write(INODES_ROOT_BLOCK, meta_data.inodes_root_block.data_as_mut());
                                disk.write(ROOT_FREE_LIST, meta_data.root_free_list.data_as_mut());
                                disk.write(INODES_FREE_LIST, meta_data.inodes_free_list.data_as_mut());
                                true
                            },
                            _ => {
                                false
                            }
                        }
                    },
                    _ => false
                }
            },
            None => false
        }
    }

    fn get_block(&mut self, block_num: usize) -> Block {
        match &mut self.disk {
            Some(disk) => {
                return fetch_block(disk, block_num);
            },
            _ => {
                return Block::new();
            }
        }
    }

    fn add_inode(&mut self, inode: Inode) -> i64 {
        // create inode block and write it to disk
        // let inodeBlock = InodeBlock::as_block();
        // disk.write(blk as usize, &mut inodeBlock.data());
        -1
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

        true
    }

    fn allocate_free_block(&mut self, inumber: usize) -> i64 {
        // NOTE: this method does not need an inumber parameter
        let meta_data = match &mut self.meta_data {
            Some(meta_data) => meta_data,
            _ => { return -1; }
        };

        let disk = match &mut self.disk {
            Some(disk) => disk,
            _ => {
                return -1;
            }
        };

        if meta_data.superblock.free_lists == 0 {
            // allocate from current_block_index
            let free_blk = meta_data.superblock.current_block_index;
            if free_blk < meta_data.superblock.blocks {
                meta_data.superblock.current_block_index += 1;
                self.save_meta_data("superblock");   // save meta_data to disk here
                return free_blk as i64;
            }
        } else {
            // read the head of the free_list
            let i = meta_data.superblock.free_lists as usize % (POINTERS_PER_BLOCK - 1);
            let mut free_block = meta_data.root_free_list.pointers();
            let free_blk = free_block[i];
            free_block[i] = 0;
            meta_data.root_free_list.set_pointers(free_block);
            meta_data.superblock.free_lists = meta_data.superblock.free_lists - 1;

            if i == 0 {
                // change the root free list here
                println!("changing root list here..");
                let next_ptr = free_block[POINTERS_PER_BLOCK - 1];
                let mut block = fetch_block(disk, next_ptr as usize);
                meta_data.root_free_list = block;

                // free next_ptr block
                self.add_free_block(next_ptr);
            }

            // remember to save meta_data here
            self.save_meta_data("all");
            return free_blk as i64;
        }
        -1
    }

    pub fn add_free_block(&mut self, block: u32) {
        // function to add to the free list
    }

    fn get_block_num(&mut self, inumber: usize, offset: usize) -> (i64, usize) {
        (-1, 0)
    }

    // fn get_block_num(&mut self, inumber: usize, offset: usize) -> (i64, usize) {
    //     // load inode info
    //     let mut inode = Inode::blank();
    //     if !self.load_inode(inumber, &mut inode) {
    //         return (-1, 0);
    //     }

    //     // compute initial block and offset index
    //     let block_index = (offset as f64 / Disk::BLOCK_SIZE as f64).floor() as usize;
    //     let block_offset = offset % Disk::BLOCK_SIZE;


    //     let get_blks = |disk: Option<&mut Disk>, ptr: u32| {
    //         let mut block = Block::new();
    //         match disk {
    //             Some(mut disc) => {
    //                 let mut blk_d = block.data();
    //                 disc.read(ptr as usize, &mut blk_d);
    //                 block.set_data(blk_d);
    //                 return block.pointers();
    //             },
    //             _ => { return block.pointers(); }
    //         }
    //     };

    //     if block_index < POINTERS_PER_INODE {
    //         if inode.direct[block_index] > 0 {
    //             // use the block number for reading
    //             return (inode.direct[block_index] as i64, block_offset)
    //         } else {
    //             return (-1, block_offset);
    //         }

    //     } else if block_index < POINTERS_PER_INODE + POINTERS_PER_BLOCK {
    //         let mut indirect_blks = get_blks(self.disk.as_mut(), inode.single_indirect);

    //         // use block index to get block number from indirect pointer
    //         let indirect_blk_index = block_index - POINTERS_PER_INODE;
    //         if indirect_blks[indirect_blk_index] > 0 {
    //             return (indirect_blks[indirect_blk_index] as i64, block_offset);
    //         } else {
    //             return (-1, block_offset);
    //         }
    //     } else if block_index < POINTERS_PER_INODE +  POINTERS_PER_BLOCK.pow(2) {
    //         let double_indirect_blks = get_blks(self.disk.as_mut(), inode.double_indirect);
    //         let i = (block_index as f64 / POINTERS_PER_BLOCK as f64).floor() as usize;
    //         if double_indirect_blks[i] > 0 {
    //             let double_indirect_blks = get_blks(self.disk.as_mut(), double_indirect_blks[i]);
    //             let j = block_index - (POINTERS_PER_INODE + POINTERS_PER_BLOCK);
    //             if double_indirect_blks[j] > 0 {
    //                 return (double_indirect_blks[j] as i64, block_offset);
    //             }
    //         }
    //         return (-1, block_offset);

    //     } else if block_index < POINTERS_PER_INODE + POINTERS_PER_BLOCK.pow(3) {
    //         let triple_indirect_blks = get_blks(self.disk.as_mut(), inode.triple_indirect);
    //         let i = (block_index as f64 / POINTERS_PER_BLOCK as f64).floor() as usize;
    //         if triple_indirect_blks[i] > 0 {
    //             let triple_indirect_blks = get_blks(self.disk.as_mut(), triple_indirect_blks[i]);
    //             let j = (block_index as f64 / POINTERS_PER_BLOCK.pow(2) as f64).floor() as usize;
    //             if triple_indirect_blks[j] > 0 {
    //                 let triple_indirect_blks = get_blks(self.disk.as_mut(), triple_indirect_blks[j]);
    //                 let k = block_index - (POINTERS_PER_INODE + POINTERS_PER_BLOCK.pow(2));
    //                 if triple_indirect_blks[k] > 0 {
    //                     return (triple_indirect_blks[k] as i64, block_offset);
    //                 }
    //             }
    //         }
    //         return (-1, block_offset);
    //     } else {
    //         return (-1, 0);
    //     }
    // }

    

    fn compute_blk_index(byte_offset: usize) -> (i64, usize) {
        // compute block and offset index
        let block_index = (byte_offset as f64 / Disk::BLOCK_SIZE as f64).floor() as usize;
        let block_offset = byte_offset % Disk::BLOCK_SIZE;
        (block_index as i64, block_offset as usize)
    }

    pub fn generate_inode_table(meta_data: &MetaData, mut disk: Disk<'a>) -> Vec<(u32, InodeBlock)> {
        let mut inode_table = Vec::new();
        let mut inode_blk = meta_data.inodes_root_block.inode_block();
        let mut block_num = INODES_ROOT_BLOCK;
        loop {
            inode_table.push((block_num as u32, inode_blk));
            if inode_blk.next_block == 0 {
                break
            }
            block_num = inode_blk.next_block as usize;
            let mut blk = Block::new();
            disk.read(block_num, blk.data_as_mut());
            inode_blk = blk.inode_block();
        }
        // println!("InodeBlock: {:?}, {}", inode_table, inode_blk.next_block);

        inode_table
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
        const number_of_blocks: usize = 120;
        const k: usize = INODES_PER_BLOCK - 1;
        const blks: usize = number_of_blocks - 3; // remove sb, rfl, and 

        let mut disk = Disk::from_file("./data/image.tfs", number_of_blocks);
        let mut fs = FileSystem::from_disk(&mut disk);
    
        const size: usize = k * blks;
        let mut inodes = [0; size];
        let mut i = 0;
        loop {
            // println!("doing: {}", i);
            if i == size {
                break;
            }
            inodes[i] = fs.create();
            i += 1;
        }
        fs.info();
        // println!("INODES: {:?}", inodes);
        // fs.remove(inode2);
        // fs.info()
        

        match fs.meta_data {
            Some(meta_data) => {
                assert_eq!(meta_data.superblock.inodes as u32, (k * blks) as u32);
            },
            None => {}
        }
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
        
        let b1 = fs.write(inode1 as usize, &mut data, 33, 0);
        // FileSystem::debug(&mut disk);
        fs.info();

        let b2 = fs.read(inode1 as usize, &mut data_r, 33, 0);

        let str1 = std::str::from_utf8(&data_r).unwrap().to_string();
        let str2 = std::str::from_utf8(&data).unwrap().to_string();

        // println!("STRING 1: {}, STRING 2: {}", str1, str2);
        // println!("bytes writen: {}, bytes read: {}", b1, b2);
    
        // assert_eq!(str1, str2);

    }
}