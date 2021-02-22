
use std::fs::{File, OpenOptions};
use std::io::prelude::*;
use std::io::SeekFrom;
use std::path::Path;
// use std::fs::OpenOptions;

#[allow(dead_code)]
pub struct Disk<'a> {
    FileDescriptor: Option<File>,
    Blocks: usize,  // Number of blocks in disk image
    Reads: usize,   // Number of reads performed
    Writes: usize,  // Number of writes performed
    Mounts: usize,  // Number of mounts
    Path: &'a str  // file path for the disk
}

impl<'a> Disk<'a> {
    pub const BLOCK_SIZE: usize = 4096;  // number of bytes per block
    fn sanity_check(&self, blocknum: usize) {
        if blocknum < 0 {
            panic!("Block Number is negative")
        }

        if blocknum >= self.Blocks {
            panic!("Block Number is too big")
        }

    }

    pub fn new() -> Disk<'a> {
        Disk {
            FileDescriptor: None,
            Blocks: 0,
            Reads: 0,
            Writes: 0,
            Mounts: 0 ,
            Path: ""
        }
    }

    pub fn from_file(path: &'a str, nblocks: usize) -> Disk<'a> {
        let mut disk = Self::new();
        disk.open(path, nblocks);
        disk
    }

    pub fn clone(&self) -> Disk<'a> {
       Self::from_file(self.Path, self.size())
    }

    /// open disk image
    /// @param path     path to disk image
    /// @param nblocks  Number of blocks in disk image
    pub fn open(&mut self, path: &'a str, nblocks: usize) {
        let path_str = path;
        let path = Path::new(path);
        let display = path.display();

        let mut file = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .open(path);
        
        self.FileDescriptor = match file {
            Ok(f) => {
                f.set_len((nblocks as u64) * (Self::BLOCK_SIZE as u64));
                Some(f)
            },
            Err(e) => {
                panic!("Could not open: {}, {}", display, e)
            }
        };

        self.Blocks = nblocks;
        self.Reads = 0;
        self.Writes = 0;
        self.Path = path_str;
    }

    pub fn size(&self) -> usize {
        self.Blocks
    }

    pub fn mount(&mut self) {
        self.Mounts = self.Mounts + 1; 
    }

    pub fn mounted(&mut self) -> bool {
        self.Mounts > 0
    }

    pub fn unmount(&mut self) {
        if self.Mounts > 0 {
            self.Mounts = self.Mounts - 1;
        }
    }

    pub fn read<'b>(&mut self, blocknum: usize, data: &'b mut [u8]) {
        self.sanity_check(blocknum);

        match self.FileDescriptor.as_mut() {
            Some(mut file) => {
                file.seek(SeekFrom::Start( blocknum as u64 * Self::BLOCK_SIZE as u64));
                file.read(data);
                self.Reads = self.Reads + 1;
            },
            None => println!("No file yet")
        }        
    }

    pub fn write<'c>(&mut self, blocknum: usize, data: &'c mut [u8]) {
        self.sanity_check(blocknum);
        match self.FileDescriptor.as_mut() {
            Some(mut file) => {
                file.seek(SeekFrom::Start(blocknum as u64 * Self::BLOCK_SIZE as u64));
                file.write(data);
                self.Writes = self.Writes + 1;
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