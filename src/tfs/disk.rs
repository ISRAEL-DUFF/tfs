
use std::fs::{File, OpenOptions};
use std::io::prelude::*;
use std::io::SeekFrom;
use std::path::Path;
// use std::fs::OpenOptions;

#[allow(dead_code)]
pub struct Disk {
    FileDescriptor: Option<File>,
    Blocks: usize,  // Number of blocks in disk image
    Reads: usize,   // Number of reads performed
    Writes: usize,  // Number of writes performed
    Mounts: usize,  // Number of mounts
}

impl Disk {
    pub const BLOCK_SIZE: usize = 4096;  // number of bytes per block
    fn sanity_check<'a>(&self, blocknum: usize, data: &'a mut [u8]) {
        if blocknum < 0 {
            panic!("Block Number is negative")
        }

        if blocknum >= self.Blocks {
            panic!("Block Number is too big")
        }

    }

    pub fn new() -> Disk {
        Disk {
            FileDescriptor: None,
            Blocks: 0,
            Reads: 0,
            Writes: 0,
            Mounts: 0 
        }
    }

    /// open disk image
    /// @param path     path to disk image
    /// @param nblocks  Number of blocks in disk image
    pub fn open(&mut self, path: &str, nblocks: usize) {
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
        println!("Open method");
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

    pub fn read<'a>(&mut self, blocknum: usize, data: &'a mut [u8]) {
        self.sanity_check(blocknum, data);

        match self.FileDescriptor.as_mut() {
            Some(mut file) => {
                file.seek(SeekFrom::Start( blocknum as u64 * Self::BLOCK_SIZE as u64));
                file.read(data);
                self.Reads = self.Reads + 1;
            },
            None => println!("No file yet")
        }        
    }

    pub fn write<'a>(&mut self, blocknum: usize, data: &'a mut [u8]) {
        self.sanity_check(blocknum, data);
        match self.FileDescriptor.as_mut() {
            Some(mut file) => {
                file.seek(SeekFrom::Start( blocknum as u64 * Self::BLOCK_SIZE as u64));
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

    #[test]
    fn disk_open() {
        let mut disk = Disk::new();
        assert_eq!(disk.open("./data/image.5", 5), ());
        assert_eq!(disk.size(), 5);
    }

    #[test]
    fn disk_read_write() {
        let mut disk = Disk::new();
        disk.open("./data/image.5", 5);
        let mut data = [3; Disk::BLOCK_SIZE];
        disk.write(1,&mut data);

        let mut data2 = [0; Disk::BLOCK_SIZE];
        disk.read(1, &mut data2);

        assert_eq!(data, data2);
    }
}