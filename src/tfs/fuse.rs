extern crate fuse;
use std::env;
use std::ffi::OsStr;
// use std::time::{Duration, UNIX_EPOCH};
use time::*;
use libc::ENOENT;
use fuse::{FileType, FileAttr, Filesystem, Request, ReplyData, ReplyEntry, ReplyAttr, ReplyDirectory};

// use super::types::*;
use crate::tfs;


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
        FileAttr {
            ino: inumber as u64,
            size: inode.size(),
            blocks: inode.total_data_blocks() as u64,
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
        }
    }

    pub fn mount(self) {
        env_logger::init();
        // let mountpoint = env::args_os().nth(1).unwrap();
        let mountpoint = "data/duffFS3";
        let options = ["-o", "ro", "-o", "fsname=DuffFS"]
            .iter()
            .map(|o| o.as_ref())
            .collect::<Vec<&OsStr>>();
        fuse::mount(self, &mountpoint, &options).unwrap();
    }
}

impl Filesystem for DuffFS<'_> {
    fn lookup(&mut self, _req: &Request, parent: u64, name: &OsStr, reply: ReplyEntry) {
        if parent == 1 {
            // let attr = self.get_attr(2);
            let n = name.to_str();
            // println!("DDDDDD: {:?}", n);
            let attr = match n {
                Some("hello.txt") => self.get_attr(2),
                Some("video.mp4") => self.get_attr(3),
                Some("audio.mp3") => self.get_attr(4),
                _ => self.get_attr(1),
            };
            // reply.entry(&TTL, &HELLO_TXT_ATTR, 0);
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
        match ino {
            1 => reply.attr(&TTL, &HELLO_DIR_ATTR),
            2 => reply.attr(&TTL, &attr),
            _ => reply.error(ENOENT),
        }
    }

    fn read(&mut self, _req: &Request, ino: u64, _fh: u64, offset: i64, size: u32, reply: ReplyData) {
        // if ino == 2 {
        //     reply.data(&HELLO_TXT_CONTENT.as_bytes()[offset as usize..]);
        // } else {
        //     reply.error(ENOENT);
        // }
        if ino >= 0 {
            let mut d: Vec<u8> = vec![0; size as usize];
            self.fs.read(ino as usize, &mut d.as_mut(), size as usize, offset as usize);
            reply.data(&d.as_slice());
        } else {
            reply.error(ENOENT);
        }
    }

    fn readdir(&mut self, _req: &Request, ino: u64, _fh: u64, offset: i64, mut reply: ReplyDirectory) {
        if ino != 1 {
            reply.error(ENOENT);
            return;
        }

        let entries = vec![
            (1, FileType::Directory, "."),
            (1, FileType::Directory, ".."),
            (2, FileType::RegularFile, "hello.txt"),
            (3, FileType::RegularFile, "video.mp4"),
            (4, FileType::RegularFile, "audio.mp3"),
        ];

        for (i, entry) in entries.into_iter().enumerate().skip(offset as usize) {
            // i + 1 means the index of the next entry
            reply.add(entry.0, (i + 1) as i64, entry.1, entry.2);
        }
        reply.ok();
    }
}