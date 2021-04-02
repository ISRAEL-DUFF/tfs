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
        println!("{} inodes", superblock.inodes);
        println!("********* END SUPER ***********\n");


        let mut i_number = 0;
        let mut end_of_tab = false;
        println!("********* INODE INFO ***********");
        let mut inode_table = Self::generate_inode_table(meta_data, disk.clone());
        let mut inode_list = InodeList::new(meta_data, &mut inode_table, disk.clone());
        let mut i: usize = 0;
        
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
        disk.write(INODES_ROOT_BLOCK, block.data_as_mut());
        disk.write(ROOT_FREE_LIST, block.data_as_mut());
        disk.write(INODES_FREE_LIST, block.data_as_mut());

        true
    }

    pub fn mount(&mut self, disk: &mut Disk<'a>) -> bool {
        let mut meta_data = Self::read_meta_data(disk);
        if meta_data.superblock.magic_number != MAGIC_NUMBER as u32 {
            return false
        }
        let disc = disk.clone();

        let inode_table = Self::generate_inode_table(&mut meta_data, disk.clone());
        self.meta_data = Some(meta_data);
        self.disk = Some(disc);
        self.inode_table = Some(inode_table);

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
            let new_blk = self.allocate_free_block();
            if !inode_list.add_inodeblock(new_blk) {
                return -1;
            }
        }
        
        
        let inumber = inode_list.add(inode);
        self.save_meta_data("all");
        inumber
    }

    pub fn remove(&mut self, inumber: usize) -> bool {
        // use inodeList to remove an inode
        // the inode list remove method should return the all 'block numbers' associated with the removed inode
        // these 'block numbers can then be added to the 'free list of block' using the 
        let mut inode = Inode::blank();
        true
    }

   pub fn stat(&mut self, inumber: usize) -> i64 {
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

        let inode = InodeProxy::new(meta_dat, inode_table, disc.clone(), inumber);
        inode.size() as i64
    }


    pub fn read(
        &mut self, inumber: usize, 
        data: &mut [u8], length: usize, offset: usize
    ) -> i64 {
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
        let mut i_list = &mut inode_list as *mut InodeList;
        let mut inode_iter = unsafe{(*i_list).read_iter(inumber)};

        // adjust length
        let initLen = length.clone();
        let mut length = length;
        let total_length = offset + length;
        let file_size = unsafe {(*i_list).get_inode(inumber).size()};
        if total_length > file_size as usize {
            // length = length - (total_length - file_size as usize);
            if offset > file_size as usize {
                return 0
            }
            length = file_size as usize - offset as usize;

            println!("initLen: {}, computedLen: {}, offset: {}, file_size: {}", initLen, length, offset, file_size);
        }

        
        let mut i = 0;
        inode_iter.seek(offset);
        for byte in inode_iter {
            if i < length {
                data[i] = byte;
                i += 1;
            } else {
                break
            }
        }
        return i as i64
        // return inode_iter.read_buffer(data, length)    
    }

    pub fn write(&mut self, inumber: usize, data: &mut [u8], length: usize, offset: usize) -> i64 {
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
        let i_list = &mut inode_list as *mut InodeList;
        let mut writer_iter = unsafe { (*i_list).write_iter(inumber) };
        let writer_i = &mut writer_iter as *mut InodeWriteIter;
        unsafe {
            if !(*writer_i).has_datablock() {
                let data_blk = (*fs_raw_ptr).allocate_free_block();
                (*writer_i).set_data_block(data_blk);
            }
        }

        let write_bytes = |data: &mut [u8], fs_ptr: *mut Self| {
            let mut i = 0;
            let mut w = unsafe { (*i_list).write_iter(inumber) };
            let writer = &mut w as *mut InodeWriteIter;

            println!("Data Len: {}", data.len());
            unsafe {
                loop {
                    if i < data.len() {
                        let r = (*writer).write_byte(data[i]);
                        if r.1 < 0 {
                            println!("WWWW: {}", i);
                            let mut data_blk = (*fs_ptr).allocate_free_block();
                            while (*writer).add_data_blk(data_blk) > 0 {
                                println!("Adding blocks");
                                data_blk = (*fs_ptr).allocate_free_block();
                            }
                            continue;
                        }
                    } else {
                        break
                    }
                    i += 1;
                }
            

                // return i as i64;

                if (*writer).flush() {
                    return i as i64
                } else {
                    -1
                }
            }
        };

        // let mut block_aligned = false;
        // let mut n_bytes_writen = 0;

        // let (aligned, n_bytes) = writer_iter.align_to_block(&mut data[offset..]);
        // let mut bytes_writen = 0;
        
        // block_aligned = aligned;
        // n_bytes_writen = n_bytes;
        // if block_aligned {
        //     let n_blocks = (length as f64 / Disk::BLOCK_SIZE as f64).floor() as usize;
        //     let mut i  = 0;
        //     let mut start = offset + n_bytes_writen as usize;
        //     let mut end = start + Disk::BLOCK_SIZE;
        //     bytes_writen += n_bytes_writen;
        //     loop {
        //         if end > data.len() && (bytes_writen as usize) < length  {
        //             end = data.len();
        //             writer_iter.flush();
        //             bytes_writen += write_bytes(&mut data[start..end], fs_raw_ptr);
        //             break
        //         }

        //         if end > length && (bytes_writen as usize) < length {
        //             end = length;
        //             writer_iter.flush();
        //             let  b = write_bytes(&mut data[start..end], fs_raw_ptr);
        //             bytes_writen += b;
        //             break
        //         }

        //         if i == n_blocks {
        //             writer_iter.flush();
        //             if (bytes_writen as usize) < length {
        //                 bytes_writen += write_bytes(&mut data[start..end], fs_raw_ptr);
        //             }
        //             break
        //         }

        //         unsafe {
        //             let success = writer_iter.write_block((*fs_raw_ptr).allocate_free_block(), &mut data[start..end]);
        //             if !success {
        //                 break
        //             }
        //         }

        //         start = end;
        //         end = start + Disk::BLOCK_SIZE;
        //         i += 1;
        //         bytes_writen += Disk::BLOCK_SIZE as i64;
        //     }

        //     return bytes_writen as i64;
        // }
        // -1

        return write_bytes(&mut data[offset..(offset+length)], fs_raw_ptr);
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


    fn allocate_free_block(&mut self) -> i64 {
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

        println!("STRING 1: {}, STRING 2: {}", str1, str2);
        println!("bytes writen: {}, bytes read: {}", b1, b2);
    
        assert_eq!(str1, str2);

    }

    #[test]
    fn test_inode_data() {
        let mut disk = Disk::from_file("./data/image.250000", 250000);
        let mut disk2 = disk.clone();
        let mut fs = FileSystem::from_disk(&mut disk);
        let mut inode_data = InodeDataPointer::new(&mut disk, 4);

        let n = 1024 + 7000;
        let depth = (((n - 5) as f64).log(POINTERS_PER_BLOCK as f64)).floor() as usize;

        for i in 5..n {
            let r = inode_data.add_data_block(i);
            if r < 0 {
                println!("Negative");
            }

            // if r > 0 {
            //     println!("Value: {}", r);
            // }
        }

        inode_data.debug();
        inode_data.save();
        assert_eq!(inode_data.indirect_ptrs.len(), depth);
        assert!(inode_data.indirect_ptrs[0].len() > 0);
        println!("indirect: {}", inode_data.indirect_ptrs[0].len());

        let mut inode_data2 = InodeDataPointer::with_depth(depth, &mut disk2, 4);
        assert_eq!(inode_data.indirect_ptrs.len(), inode_data2.indirect_ptrs.len());
        assert_eq!(inode_data.indirect_ptrs[0].len(), inode_data2.indirect_ptrs[0].len());
        assert_eq!(inode_data.indirect_ptrs[0][2], inode_data2.indirect_ptrs[0][2]);

    }
}