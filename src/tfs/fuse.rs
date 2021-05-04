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
use libc::ENOENT;
use fuse::{
    FileType, FileAttr, Filesystem, Request, ReplyData, ReplyWrite, 
    ReplyCreate, ReplyEntry, ReplyAttr, ReplyDirectory, ReplyOpen, ReplyEmpty
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
        encoded
    }

    pub fn deserialize(v: Vec<u8>) -> Self {
        // bincode::deserialize(&v[..]).unwrap()
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

const HELLO_DIR_ATTR: FileAttr = FileAttr {
    ino: 1,
    size: 0,
    blocks: 0,
    atime: UNIX_EPOCH,                                  // 1970-01-01 00:00:00
    mtime: UNIX_EPOCH,
    ctime: UNIX_EPOCH,
    crtime: UNIX_EPOCH,
    kind: FileType::Directory,
    perm: 0o755,
    nlink: 2,
    uid: 501,
    gid: 20,
    rdev: 0,
    flags: 0,
};

const HELLO_TXT_CONTENT: &str = "Hello World!\n";

const HELLO_TXT_ATTR: FileAttr = FileAttr {
    ino: 2,
    size: 13,
    blocks: 1,
    atime: UNIX_EPOCH,                                  // 1970-01-01 00:00:00
    mtime: UNIX_EPOCH,
    ctime: UNIX_EPOCH,
    crtime: UNIX_EPOCH,
    kind: FileType::RegularFile,
    perm: 0o644,
    nlink: 1,
    uid: 501,
    gid: 20,
    rdev: 0,
    flags: 0,
};

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
            perm: 777,
            nlink: 1,
            uid: 501,
            gid: 20,
            rdev: 0,
            flags: 0,
        }
    }

    pub fn dir_from(&mut self, inumber: u32) -> Directory {
        // println!("Inumber: {}", inumber);
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
                content.extend_from_slice(&mut buffer);           
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
        Directory::deserialize(content)
    }

    pub fn dir_entries(&mut self, inumber: u64) -> Vec<(u64, FileType, String)> {
        let directory = self.dir_from(inumber as u32);
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
    
            let mut data = dir.serialize();
            let size = data.len();
            (*fs).truncate(inumber as usize, data.len());
            let mut inode_list = (*fs).inode_list();
            let mut write_obj = inode_list.write_iter(inumber as usize);
            let write = &mut write_obj as *mut tfs::InodeWriteIter;
            (*write).seek(0);
            (*fs).write_data(write_obj, data.as_mut(), size, 0);
            inumber as u32
        }
    }

    pub fn mount(self) {
        env_logger::init();
        // let mountpoint = env::args_os().nth(1).unwrap();
        let mountpoint = "data/mnt2";
        let options = ["-o", "rw", "-o", "fsname=DuffFS"]
            .iter()
            .map(|o| o.as_ref())
            .collect::<Vec<&OsStr>>();
        fuse::mount(self, &mountpoint, &options).unwrap();
    }
}

impl Filesystem for DuffFS<'_> {
    fn lookup(&mut self, _req: &Request, parent: u64, name: &OsStr, reply: ReplyEntry) {
        // println!("Parent: {}", parent);
        let attr = self.get_attr(parent as usize);
        // println!("Parent: {:?}", attr);

        if true {   // TODO: check if parent exist
            // // let attr = self.get_attr(2);
            // let n = name.to_str();
            // // println!("DDDDDD: {:?}", n);
            // let attr = match n {
            //     Some("hello.txt") => self.get_attr(2),
            //     Some("video.mp4") => self.get_attr(3),
            //     Some("audio.mp3") => self.get_attr(4),
            //     _ => self.get_attr(1),
            // };
            // // reply.entry(&TTL, &HELLO_TXT_ATTR, 0);
            reply.entry(&TTL, &attr, 0);
        } else {
            reply.error(ENOENT);
        }
    }

    fn getattr(&mut self, _req: &Request, ino: u64, reply: ReplyAttr) {
        let attr = self.get_attr(ino as usize);
        // match ino {
        //     1 => reply.attr(&TTL, &HELLO_DIR_ATTR),
        //     2 => reply.attr(&TTL, &HELLO_TXT_ATTR),
        //     _ => reply.error(ENOENT),
        // }

        // match ino {
        //     1 => reply.attr(&TTL, &HELLO_DIR_ATTR),
        //     2 => reply.attr(&TTL, &attr),
        //     _ => reply.error(ENOENT),
        // }
        // reply.attr(&TTL, &HELLO_DIR_ATTR)
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
        println!("Create Called: {:?}", name);
        let inumber = self.create_file(name, FileType::RegularFile, parent as u32);
        let attr = self.get_attr(inumber as usize);
        reply.created(&TTL, &attr, 0, 0, 0);
    }

    fn read(&mut self, _req: &Request, ino: u64, _fh: u64, offset: i64, size: u32, reply: ReplyData) {
        // if ino == 2 {
        //     reply.data(&HELLO_TXT_CONTENT.as_bytes()[offset as usize..]);
        // } else {
        //     reply.error(ENOENT);
        // }
        if ino > 0 {
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
        println!("WRITE CALLED");
        if ino > 0 {
            let mut dat = data;
            let n = self.fs.write(ino as usize, &mut dat, dat.len(), offset as usize);
            reply.written(n as u32);
        } else {
            reply.error(0);
        }
    }

    fn readdir(&mut self, _req: &Request, ino: u64, _fh: u64, offset: i64, mut reply: ReplyDirectory) {
        // if ino == 0 {
        //     reply.error(ENOENT);
        //     return;
        // }

        // let entries = vec![
        //     (1, FileType::Directory, "."),
        //     (1, FileType::Directory, ".."),
        //     (2, FileType::RegularFile, "hello.txt"),
        //     (3, FileType::RegularFile, "video.mp4"),
        //     (4, FileType::RegularFile, "audio.mp3"),
        // ];
        let entries = self.dir_entries(ino);

        for (i, entry) in entries.into_iter().enumerate().skip(offset as usize) {
            // i + 1 means the index of the next entry
            reply.add(entry.0, (i + 1) as i64, entry.1, entry.2);
        }
        reply.ok();
    }

    fn flush(&mut self, _req: &Request, _ino: u64, _fh: u64, _lock_owner: u64, reply: ReplyEmpty) {
        reply.ok()
    }
}