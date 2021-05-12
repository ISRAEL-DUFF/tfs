extern crate fuse;
// use std::env;
use std::{
    env,
    collections::BTreeMap,
    ffi::OsString,
    ffi::OsStr,
    path::Path,
};

// use std::time::{Duration, UNIX_EPOCH};
use time::*;
use libc::{ENOENT, c_int};
use fuse::{
    FileType, FileAttr, Filesystem, Request, ReplyData, ReplyWrite, 
    ReplyCreate, ReplyEntry, ReplyAttr, ReplyDirectory, ReplyOpen, ReplyEmpty, ReplyStatfs
};

extern crate bincode;
// use bincode::{serialize, deserialize, Bounded};
use serde::{Deserialize, Serialize};

// use super::types::*;
use crate::tfs;

#[derive(Serialize, Deserialize, Debug, Default)]
pub struct Directory {
    pub entries: BTreeMap<OsString, u32>    // <pathname, inumber>
}


impl Directory {
    pub fn entry<P>(&self, path: P) -> Result<u32, bool>
    where
        P: AsRef<Path>,
    {
        self.entries
            .get(&path.as_ref().as_os_str().to_os_string())
            .ok_or(false)
            .map(|x| *x)
    }

    pub fn new(inumber: u32, parent: u32) -> Self {
        let mut entries = BTreeMap::new();
        entries.insert(OsString::from("."), inumber);
        entries.insert(OsString::from(".."), parent);
        Self {
            entries
        }
    }

    pub fn add_entry<P>(&mut self, path: P, inumber: u32) where P: AsRef<Path> {
        self.entries.insert(path.as_ref().as_os_str().to_os_string(), inumber);
    }

    pub fn remove_entry<P>(&mut self, path: P) where P: AsRef<Path> {
        self.entries.remove(&path.as_ref().as_os_str().to_os_string());
    }

    pub fn find_by_path<P>(&self, path: P) -> Result<u32, bool> 
    where P: AsRef<Path> 
    {
        self.entry(path)
    }

    pub fn serialize(&self) -> Vec<u8> {
        // let limit = Bounded(20);
        let encoded: Vec<u8> = bincode::serialize(self).unwrap();
        let encoded2: Vec<u8> = bincode::serialize(self).unwrap();
        println!("serialized - CHECK: {:?}", Directory::deserialize(encoded2));
        encoded
    }

    pub fn deserialize(v: Vec<u8>) -> Self {
        // bincode::deserialize(&v[..]).unwrap()
        // println!("AGAIN: {:?}", v);
        match bincode::deserialize(&v[..]) {
            Ok(dir) => dir,
            Err(e) => {
                // panic!("Not available")
                println!("Not available");
                Self::new(1, 1)
            }
        }
    }

}


// const TTL: Duration = Duration::from_secs(1);           // 1 second
const TTL: Timespec = Timespec { sec: 1, nsec: 0 };
const UNIX_EPOCH: Timespec = Timespec { sec: 0, nsec: 0 };


pub struct DuffFS<'a> {
    fs: tfs::FileSystem<'a>
}

impl<'a> DuffFS<'a> {
    pub fn new(fs: tfs::FileSystem<'a>) -> Self {
        Self {
            fs
        }
    }

    pub fn get_attr(&mut self, inumber: usize) -> FileAttr {
        let inode = self.fs.get_inode(inumber);
        let mut kind = FileType::RegularFile;
        if inode.is_dir() {
            kind = FileType::Directory;
        }
        FileAttr {
            ino: inumber as u64,
            size: inode.size(),
            blocks: inode.total_data_blocks() as u64,
            atime: UNIX_EPOCH,                                  // 1970-01-01 00:00:00
            mtime: UNIX_EPOCH,
            ctime: UNIX_EPOCH,
            crtime: UNIX_EPOCH,
            kind,
            perm: 0o777,
            nlink: 1,
            uid: 501,
            gid: 20,
            rdev: 0,
            flags: 0,
        }
    }

    pub fn dir_from(&mut self, inumber: u32) -> Directory {
        // 1. read the whole content of inode
        let mut content: Vec<u8> = vec![];
        let mut buffer = [0; tfs::constants::BUFFER_SIZE];
        let mut offset = 0;
        loop {
            let result = self.fs.read(inumber as usize, &mut buffer, tfs::constants::BUFFER_SIZE, offset);
            if result <= 0 {
                break;
            }

            if result < tfs::constants::BUFFER_SIZE as i64 {
                let mut buff = &buffer[0..result as usize];
                content.extend_from_slice(&mut buff); 
                break;          
            } else {
                content.extend_from_slice(&mut buffer);
            }
            offset += result as usize;
        }

        // 2. deserialize content
        // match Directory::deserialize(content) {
        //     Ok(directory) => directory,
        //     Err(e) => Directory::new(inumber, inumber)
        // }
        // println!("Content Len: {}", content.len());
        let directory = Directory::deserialize(content);

        println!("Dir from: {}, {:?}", inumber, directory);
        directory
    }

    pub fn dir_entries(&mut self, inumber: u64) -> Vec<(u64, FileType, String)> {
        let directory = self.dir_from(inumber as u32);
        println!("Entries from dir {}, {:?}", inumber, directory);
        let mut entries: Vec<(u64, FileType, String)> = Vec::new();
        for (path, inum) in directory.entries {
            if self.fs.get_inode(inum as usize).is_dir() {
                entries.push(
                    (inum as u64, FileType::Directory, path.into_string().unwrap())
                );
            } else {
                entries.push(
                    (inum as u64, FileType::RegularFile, path.into_string().unwrap())
                );
            }
            // println!("{:?}: \"{}\"", path, inumber);
        }
        entries
    }

    pub fn save_dir(&mut self, dir: Directory, inumber: usize) {
        let fs = &mut self.fs as *mut tfs::FileSystem;
        unsafe {
            let mut data = dir.serialize();
            let size = data.len();
            // (*fs).truncate(parent as usize, data.len());
            let mut inode_list = (*fs).inode_list();
            let mut write_obj = inode_list.write_iter(inumber);
            let write = &mut write_obj as *mut tfs::InodeWriteIter;
            (*write).seek(0);
            let n = (*fs).write_data(write_obj, data.as_mut(), size, 0);
            println!("Number of bytes written: {}", n);
        }
    }

    pub fn create_file(&mut self, name: &OsStr, kind: FileType, parent: u32) -> u32 {
        let fs = &mut self.fs as *mut tfs::FileSystem;
        let mut dir = self.dir_from(parent);
        let mut inode = tfs::Inode::blank();
        inode.kind = match kind {
            FileType::RegularFile => 1,
            FileType::Directory => 2,
            _ => panic!("Unknown file type")
        };
        unsafe {
            let inumber = (*fs).create_from(inode);
            dir.add_entry(Path::new(&name.to_str().unwrap()), inumber as u32);
            println!("Added Entry to Directory: {}, {:?}", parent, dir);
            self.save_dir(dir, parent as usize);
            if kind == FileType::Directory {
                self.save_dir(Directory::new(inumber as u32, parent as u32), inumber as usize)
            }
            inumber as u32
        }
    }

    pub fn mount(self) {
        // env_logger::init();
        // let mountpoint = env::args_os().nth(1).unwrap();
        let mountpoint = "data/duffFS3";
        let options = ["-o", "rw", "-o", "fsname=DuffFS", "-o", "volname=DuffFS"]
            .iter()
            .map(|o| o.as_ref())
            .collect::<Vec<&OsStr>>();
        fuse::mount(self, &mountpoint, &options).unwrap();
    }
}

impl Filesystem for DuffFS<'_> {
    fn init(&mut self, _req: &Request<'_>) -> Result<(), c_int> {
        if !self.fs.inode_list().inode_exists(1) {
            let this = self as *mut DuffFS;
            unsafe {
                let inumber = (*this).fs.create_dir();
                self.save_dir(Directory::new(inumber as u32, inumber as u32), inumber as usize);
                Ok(())
            }
        } else {
            Ok(())
        }
    }

    fn lookup(&mut self, _req: &Request, parent: u64, name: &OsStr, reply: ReplyEntry) {
        let dir = self.dir_from(parent as u32);

        match dir.find_by_path(name) {   // TODO: check if parent exist
            Ok(inum) => {
                let attr = self.get_attr(inum as usize);
                println!("Look Up: {:?}", name);
                reply.entry(&TTL, &attr, 0);
            },
            Err(e) => {
                reply.error(ENOENT);
            }
        }
    }

    fn getattr(&mut self, _req: &Request, ino: u64, reply: ReplyAttr) {
        let attr = self.get_attr(ino as usize);
        reply.attr(&TTL, &attr)
    }

    fn setattr(
        &mut self, 
        _req: &Request, 
        ino: u64, 
        _mode: Option<u32>, 
        _uid: Option<u32>, 
        _gid: Option<u32>, 
        _size: Option<u64>, 
        _atime: Option<Timespec>, 
        _mtime: Option<Timespec>, 
        _fh: Option<u64>, 
        _crtime: Option<Timespec>, 
        _chgtime: Option<Timespec>, 
        _bkuptime: Option<Timespec>, 
        _flags: Option<u32>, 
        reply: ReplyAttr
    ) { 
        let attr = self.get_attr(ino as usize);
        reply.attr(&TTL, &attr)
     }

    fn open(&mut self, _req: &Request, ino: u64, flags: u32, reply: ReplyOpen) {
        println!("OPEN called");
        reply.opened(ino, flags);
    }

    fn opendir(&mut self, _req: &Request, ino: u64, flags: u32, reply: ReplyOpen) {
        println!("OPENDIR called");
        reply.opened(ino, flags);
    }

    fn mknod<'a>(
        &mut self,
        _req: &Request<'_>,
        parent: u64,
        name: &OsStr,
        _mode: u32,
        _rdev: u32,
        reply: ReplyEntry
    ) {
        println!("Make NODE: {:?}", name);
        let inumber = self.create_file(name, FileType::RegularFile, parent as u32);
        let attr = self.get_attr(inumber as usize);
        reply.entry(&TTL, &attr, 0);
    }

    fn mkdir(
        &mut self,
        _req: &Request<'_>,
        parent: u64,
        name: &OsStr,
        _mode: u32,
        reply: ReplyEntry
    ) {
        println!("Make dir: {:?}", name);
        let inumber = self.create_file(name, FileType::Directory, parent as u32);
        let attr = self.get_attr(inumber as usize);
        reply.entry(&TTL, &attr, 0);
    }

    fn create(
        &mut self,
        _req: &Request<'_>,
        parent: u64,
        name: &OsStr,
        _mode: u32,
        _flags: u32,
        reply: ReplyCreate
    ) {
        let inumber = self.create_file(name, FileType::RegularFile, parent as u32);
        let attr = self.get_attr(inumber as usize);
        reply.created(&TTL, &attr, 0, 0, 0);
    }

    fn unlink(
        &mut self,
        _req: &Request<'_>,
        parent: u64,
        name: &OsStr,
        reply: ReplyEmpty
    ) {
        let mut dir = self.dir_from(parent as u32);

        match dir.find_by_path(name) {   // TODO: check if parent exist
            Ok(inum) => {
                let attr = self.fs.remove(inum as usize);
                println!("Remove File: {}, with name => {:?}", inum, name);
                dir.remove_entry(name);
                self.save_dir(dir, parent as usize);
                reply.ok();
            },
            Err(e) => {
                reply.error(ENOENT);
            }
        }
    }

    fn rmdir(
        &mut self,
        _req: &Request<'_>,
        _parent: u64,
        _name: &OsStr,
        reply: ReplyEmpty
    ) {
        reply.ok()
    }

    fn read(&mut self, _req: &Request, ino: u64, _fh: u64, offset: i64, size: u32, reply: ReplyData) {
        if ino > 0 {
            // println!("READ => byte_offset: {}, length: {}", offset, size);
            let mut d: Vec<u8> = vec![0; size as usize];
            self.fs.read(ino as usize, &mut d.as_mut(), size as usize, offset as usize);
            reply.data(&d.as_slice());
        } else {
            reply.error(ENOENT);
        }
    }

    fn write(
        &mut self,
        _req: &Request<'_>,
        ino: u64,
        _fh: u64,
        offset: i64,
        data: &[u8],
        _flags: u32,
        reply: ReplyWrite
    ) {
        if ino > 0 {
            let mut dat = data;
            // let n = self.fs.write(ino as usize, &mut dat, dat.len(), offset as usize);
            let n = self.fs.write(ino as usize, &mut dat, dat.len(), 0);
            reply.written(n as u32);
        } else {
            reply.error(0);
        }
    }

    fn readdir(&mut self, _req: &Request, ino: u64, _fh: u64, offset: i64, mut reply: ReplyDirectory) {
        let entries = self.dir_entries(ino);
        println!("Directory Entries: {}, {:?}, offset: {}", ino, entries, offset);
        for (i, entry) in entries.into_iter().enumerate().skip(offset as usize) {
            // i + 1 means the index of the next entry
            reply.add(entry.0, (i + 1) as i64, entry.1, entry.2);
        }
        reply.ok();
    }

    fn flush(&mut self, _req: &Request, _ino: u64, _fh: u64, _lock_owner: u64, reply: ReplyEmpty) {
        reply.ok()
    }

    // setvolname(&mut self, _req: &Request<'_>, _name: &OsStr, reply: ReplyEmpty) {

    // }

    fn statfs(&mut self, _req: &Request<'_>, ino: u64, reply: ReplyStatfs) {
        println!("StatFS: {}", ino);
        match self.fs.meta_data {
            Some(meta_data) => {
                let bavail = meta_data.superblock.blocks - meta_data.superblock.current_block_index;
                reply.statfs(
                    meta_data.superblock.blocks as u64,
                    bavail as u64,
                    bavail as u64,
                    5,
                    8,
                    4096,
                    300,
                    4096
                )
            },
            None => {}
        }
        // pub fn statfs(
        //     self,
        //     blocks: u64,
        //     bfree: u64,
        //     bavail: u64,
        //     files: u64,
        //     ffree: u64,
        //     bsize: u32,
        //     namelen: u32,
        //     frsize: u32
        // )
        
        
    }

    fn rename(
        &mut self,
        _req: &Request<'_>,
        parent: u64,
        name: &OsStr,
        _newparent: u64,
        newname: &OsStr,
        reply: ReplyEmpty
    ) {
        println!("RENaem called: {:?}, {:?}", name, newname);
        let mut parent_dir = self.dir_from(parent as u32);

        // for (&mut path, inum) in parent_dir.entries.iter_mut() {
        //     if *path == name {
        //         *path = newname.to_os_string();
        //     }
        // }

        let inum = parent_dir.find_by_path(name).unwrap();
        parent_dir.remove_entry(name);
        parent_dir.add_entry(newname, inum);
        self.save_dir(parent_dir, parent as usize);
        reply.ok();
    }

}