mod disk;
mod constants;
mod types;
mod utility;
mod fuse;

use self::disk::Disk;
use self::constants::*;
use self::types::*;
use self::utility::*;
use self::prelude::block::BlockManager;

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
        println!("{} Deallocated blocks", superblock.free_blocks);
        println!("{} Free Inodes", superblock.free_inodes);
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
                current_block_index: 4,
                free_blocks: 0,
                free_inodes: 0,
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

    pub fn create<'c:'a>(&'c mut self) -> i64 {
        // create the inode
        let inumber = self.inode_list().add(Inode::blank());
        self.save_meta_data("all");
        inumber
    }

    pub fn create_from<'c:'a>(&'c mut self, inode: Inode) -> i64 {
        // create the inode
        let inumber = self.inode_list().add(inode);
        self.save_meta_data("all");
        inumber
    }

    pub fn create_dir<'c:'a>(&'c mut self) -> i64 {
        // create the inode
        let mut inode = Inode::blank();
        inode.kind = 2;
        let inumber = self.inode_list().add(inode);
        self.save_meta_data("all");
        inumber
    }

    pub fn remove(&mut self, inumber: usize) -> bool {
        // use inodeList to remove an inode
        self.inode_list().remove(inumber)
    }

   pub fn stat(&mut self, inumber: usize) -> i64 {
        self.get_inode(inumber).size() as i64
    }


    pub fn read(
        &mut self, inumber: usize, 
        data: &mut [u8], length: usize, offset: usize
    ) -> i64 {
        let this = self as *mut Self;
        let mut inode_list = self.inode_list();
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

            // println!("initLen: {}, computedLen: {}, offset: {}, file_size: {}", initLen, length, offset, file_size);
        }

        
        let i_iter = &mut inode_iter as *mut InodeReadIter;
        unsafe{(*i_iter).seek(offset)};
        // let mut i = 0;
        // for byte in inode_iter {
        //     if i < length {
        //         data[i] = byte;
        //         i += 1;
        //     } else {
        //         break
        //     }
        // }
        // println!("returned i: {}", i);
        // return i as i64
        return inode_iter.read_buffer(data, length)    
    }

    pub fn write_data(&mut self, mut write_obj: InodeWriteIter, data: &[u8], length: usize, offset: usize) -> i64 {
        let fs_raw_ptr = self as *mut Self;

        // let mut inode_list = self.inode_list();
        // let i_list = &mut inode_list as *mut InodeList;
        // let mut writer_iter = unsafe { (*i_list).write_iter(inumber) };
        // let writer_i = &mut writer_iter as *mut InodeWriteIter;

        let writer = &mut write_obj as *mut InodeWriteIter;

        let write_bytes = |data: &[u8], w: *mut InodeWriteIter| {
            let mut i = 0;
            // let mut w = unsafe { (*i_list).write_iter(inumber) };
            // let writer = &mut w as *mut InodeWriteIter;

            println!("Data Len: {}", data.len());
            unsafe {
                loop {
                    if i < data.len() {
                        let r = (*w).write_byte(data[i]);
                        if r.1 < 0 {
                            return -1;
                        }
                    } else {
                        break
                    }
                    i += 1;
                }
            
                return i as i64;

                // if (*w).flush() {
                //     return i as i64
                // } else {
                //     -1
                // }
            }
        };

        let mut block_aligned = false;
        let mut n_bytes_writen = 0;

        let (aligned, n_bytes) = unsafe { (*writer).align_to_block(&data[offset..], length) };
        let mut bytes_writen = 0;
        
        block_aligned = aligned;
        n_bytes_writen = n_bytes;
        if block_aligned {
            let len = (length - n_bytes_writen as usize) as f64;
            let n_blocks = (len / Disk::BLOCK_SIZE as f64).floor() as usize;
            let mut i  = 0;
            let mut start = offset + n_bytes_writen as usize;
            let mut end = start + Disk::BLOCK_SIZE;
            bytes_writen += n_bytes_writen;
            loop {
                if end > data.len() && (bytes_writen as usize) < length  {
                    end = data.len();
                    // unsafe {(*writer).flush()};
                    bytes_writen += write_bytes(&data[start..end], writer);
                    break
                }

                if end > length && (bytes_writen as usize) < length {
                    end = length;
                    // unsafe {(*writer).flush()};
                    let  b = write_bytes(&data[start..end], writer);
                    bytes_writen += b;
                    break
                }

                if i == n_blocks {
                    // unsafe {(*writer).flush()};
                    if (bytes_writen as usize) < length {
                        bytes_writen += write_bytes(&data[start..end], writer);
                    }
                    break
                }

                unsafe {
                    let r = (*writer).write_block(&data[start..end]);
                    if r < 0 {
                        break;
                    }
                }

                start = end;
                end = start + Disk::BLOCK_SIZE;
                i += 1;
                bytes_writen += Disk::BLOCK_SIZE as i64;
            }

            // temporary code for debug purposes
            // unsafe {
            //     match (*writer_i).get_inode().data_manager() {
            //         Some(data_man) => data_man.debug(),
            //         _ => println!("Nothing")
            //     }
            // };
            unsafe {(*writer).flush()};
            return bytes_writen as i64;
        }
        -1

        // return write_bytes(&mut data[offset..(offset+length)], fs_raw_ptr);
    }

    pub fn write(&mut self, inumber: usize, data: &[u8], length: usize, offset: usize) -> i64 {
        let this = self as *mut FileSystem;
        unsafe {
            let mut inode_list = (*this).inode_list();
            (*this).write_data(inode_list.write_iter(inumber), data, length, offset)
        }
    }

    pub fn truncate(&mut self, inumber: usize, byte_offset: usize) {
        let this = self as *mut Self;

        let (meta_dat, disc, inode_table) = unsafe {
            match resolve_attr(&mut (*this)) {
                Some(attr) => attr,
                None => {
                    return;
                }
            }
        };

        let mut inod = self.get_inode(inumber);
        let mut inode = &mut inod as *mut InodeProxy;
        
        unsafe {
            let mut l = 0;
            let mut unused_blocks = vec![];
            let mut n_free_direct_ptrs = 0;
            if (*inode).init_datablocks() {
                let r = match (*inode).data_manager_mut() {
                    Some(data_manager) => data_manager.truncate(byte_offset),
                    None => (0, 0, vec![])
                };

                l = r.0;
                n_free_direct_ptrs = r.1;
                unused_blocks = r.2;
            }
            

            println!("TRuncate Len: {}", l);
    
            if l < 0 {
                let total_data_blocks = (*inode).total_data_blocks();
                // println!("Traaa: {}, {}, {}", total_data_blocks, unused_blocks.len(), n_free_direct_ptrs);
                (*inode).set_data_blocks_count(total_data_blocks as usize - n_free_direct_ptrs);
                (*inode).save_data_blocks();
                let mut block_mgr = BlockManager::new(meta_dat, disc.clone());
                if byte_offset == 0 {
                    // block_mgr.free_blocks(vec![(*inode).data_block()]);
                    unused_blocks.push((*inode).data_block());
                    (*inode).set_data_block(0);
                }
                block_mgr.free_blocks(unused_blocks);

                // set new size
                (*inode).set_size(byte_offset);
                (*inode).save();
            }

            let size = (*inode).size() as usize;
            let mut len = 0;
            if byte_offset > size {
                len = byte_offset - size;
            } else {
                (*inode).set_size(byte_offset);
                (*inode).save();
            }
    
            if l >= 0 && len > 0 {
                loop {
                    let mut data = [0; BUFFER_SIZE];
                    if len < BUFFER_SIZE {
                        self.write(inumber, &mut data, len, 0);
                        break;
                    } else {
                        self.write(inumber, &mut data, BUFFER_SIZE, 0);
                        len -= BUFFER_SIZE;
                    }
                }
            }
        }
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

    fn save_meta_data<'c:'a>(&'c mut self, name: &str) -> bool {
        match &mut self.meta_data {
            Some(meta_data) => {
                match &mut self.disk {
                    Some(disk) => {
                        save_meta_data(meta_data, disk, name)
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

    pub fn inode_list(&mut self) -> InodeList {
        let this = self as *mut Self;
        let (meta_dat, disc, inode_table) = unsafe {
            match resolve_attr(&mut (*this)) {
                Some(attr) => attr,
                None => {
                    panic!("could not find attribute");
                }
            }
        };
        InodeList::new(meta_dat, inode_table, disc.clone())
    }

    // pub fn write_obj(&mut self, inumber) -> {
    //     self.inode_list().write_iter(inumber)
    // }

    pub fn get_inode(&mut self, inumber: usize) -> InodeProxy {
        let this = self as *mut Self;
        let (meta_dat, disc, inode_table) = unsafe {
            match resolve_attr(&mut (*this)) {
                Some(attr) => attr,
                None => {
                    panic!("could not find attribute");
                }
            }
        };

        InodeProxy::new(meta_dat, inode_table, disc.clone(), inumber)
    }

    pub fn get_inodes(&mut self) -> Vec<InodeProxy> {
        let this = self as *mut Self;
        let mut inode_list = self.inode_list();
        let mut inodes = Vec::new();
        let mut inum = 1;
        for inode in inode_list.iter() {
            if inode.valid == 1 {
                unsafe {
                    inodes.push((*this).get_inode(inum));
                }
            }
            inum += 1;
        }
        inodes
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
    pub use super::fuse::*;
    pub use super::constants::*;
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

    // #[test]
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
        let mut inode_data = InodeDataPointer::new(disk, 4);

        let n = 1024 + 7000;
        // let n = 1054;
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
        println!("indirect: {}, depth: {}", inode_data.indirect_ptrs[0].len(), depth);

        let mut inode_data2 = InodeDataPointer::with_depth(depth, disk2, 4);
        assert_eq!(inode_data.direct_ptrs.len(), inode_data2.direct_ptrs.len());
        assert_eq!(inode_data.indirect_ptrs.len(), inode_data2.indirect_ptrs.len());
        assert_eq!(inode_data.indirect_ptrs[0].len(), inode_data2.indirect_ptrs[0].len());
        assert_eq!(inode_data.indirect_ptrs[0][1], inode_data2.indirect_ptrs[0][1]);

    }
}