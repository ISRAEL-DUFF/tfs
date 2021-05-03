
use std::fs::{File, OpenOptions};
use std::io::prelude::*;
use std::io::SeekFrom;
use std::path::Path;
// use std::fs::OpenOptions;

#[allow(dead_code)]
pub struct Disk<'a> {
    file_descriptor: Option<File>,
    blocks: usize,  // Number of blocks in disk image
    reads: usize,   // Number of reads performed
    writes: usize,  // Number of writes performed
    mounts: usize,  // Number of mounts
    pub path: &'a str  // file path for the disk
}

impl<'a> Disk<'a> {
    pub const BLOCK_SIZE: usize = 4096;  // number of bytes per block

    fn sanity_check(&self, blocknum: usize) {
        if blocknum < 0 {
            panic!("Block Number is negative")
        }

        if blocknum >= self.blocks {
            panic!("Block Number is too big")
        }

    }

    pub fn new() -> Disk<'a> {
        Disk {
            file_descriptor: None,
            blocks: 0,
            reads: 0,
            writes: 0,
            mounts: 0 ,
            path: ""
        }
    }

    pub fn from_file(path: &'a str, nblocks: usize) -> Disk<'a> {
        let mut disk = Self::new();
        disk.open(path, nblocks);
        disk
    }

    pub fn clone(&self) -> Disk<'a> {
       Self::from_file(self.path, self.size())
    }

    /// open disk image
    /// @param path     path to disk image
    /// @param nblocks  Number of blocks in disk image
    pub fn open(&mut self, path: &'a str, nblocks: usize) {
        let path_str = path;
        let path = Path::new(path);
        let display = path.display();

        let file = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .open(path);
        
        self.file_descriptor = match file {
            Ok(f) => {
                f.set_len((nblocks as u64) * (Self::BLOCK_SIZE as u64));
                Some(f)
            },
            Err(e) => {
                panic!("Could not open: {}, {}", display, e)
            }
        };

        self.blocks = nblocks;
        self.reads = 0;
        self.writes = 0;
        self.path = path_str;
    }

    pub fn size(&self) -> usize {
        self.blocks
    }

    pub fn mount(&mut self) {
        self.mounts = self.mounts + 1; 
    }

    pub fn mounted(&mut self) -> bool {
        self.mounts > 0
    }

    pub fn unmount(&mut self) {
        if self.mounts > 0 {
            self.mounts = self.mounts - 1;
        }
    }

    pub fn read<'b>(&mut self, blocknum: usize, data: &'b mut [u8]) {
        self.sanity_check(blocknum);

        match self.file_descriptor.as_mut() {
            Some(file) => {
                file.seek(SeekFrom::Start(blocknum as u64 * Self::BLOCK_SIZE as u64));
                file.read(data);
                self.reads = self.reads + 1;
            },
            None => println!("No file yet")
        }        
    }

    pub fn write<'c>(&mut self, blocknum: usize, data: &'c [u8]) {
        self.sanity_check(blocknum);
        match self.file_descriptor.as_mut() {
            Some(file) => {
                file.seek(SeekFrom::Start(blocknum as u64 * Self::BLOCK_SIZE as u64));
                file.write(data);
                self.writes = self.writes + 1;
            },
            None => println!("No file yet")
        }        
    }
}


#[cfg(test)]
mod tests {
    use super::*;

    // #[test]
    fn disk_open() {
        let mut disk = Disk::new();
        assert_eq!(disk.open("./data/image.5", 5), ());
        assert_eq!(disk.size(), 5);
    }

    // #[test]
    fn disk_read_write() {
        let mut disk = Disk::new();
        disk.open("./data/image.5", 5);
        let mut data = [3; Disk::BLOCK_SIZE];
        disk.write(1,&mut data);

        let mut data2 = [0; Disk::BLOCK_SIZE];
        disk.read(1, &mut data2);

        assert_eq!(data, data2);
    }

    // #[test]
    fn disk_clone() {
        let mut disk = Disk::new();
        disk.open("./data/image.50", 50);
        let mut data = [3; Disk::BLOCK_SIZE];
        disk.write(1,&mut data);

        let mut disk_clone = disk.clone();

        let mut data2 = [4; Disk::BLOCK_SIZE];
        disk_clone.read(1,&mut data2);

        // compare values
        assert_eq!(data, data2);
    }
}