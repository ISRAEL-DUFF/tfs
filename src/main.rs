use std::{env, process};
use std::io::{stdin, stdout, Write};
use std::io::SeekFrom;
use std::io::prelude::*;
use std::error::Error;
use disk::prelude::*;

fn main() {
    let args: Vec<String> = env::args().collect();

    if args.len() != 3 {
        eprintln!("Usage: {}, <diskfile> <nblocks>", args[0]);
        process::exit(1);
    }

    let mut fs = FileSystem::new();
    let nblocks: usize = match args[2].as_str().parse() {
        Ok(n) => n,
        _ => {
            println!("Invalid number of blocks {}", args[2]);
            process::exit(1);
        }
    };
    let mut disk = Disk::from_file(&args[1], nblocks);

    // shell loop
    loop {
        let line = read_command();
        let command = parse_command(line.as_str());
        if command.len() == 0 {
            continue;
        }
        let cmd = command[0];

        if cmd == "help" {
            do_help();
        } else if cmd == "format" {
            disk = do_format(disk, command);
        } else if cmd == "debug" {
            disk = do_debug(disk, command);
        } else if cmd == "mount" {
            let r = do_mount(disk, fs, command);
            disk = r.0;
            fs = r.1;
        } else if cmd == "fuse_mount" {
            let r = do_fuse_mount(disk, fs, command);
            disk = r.0;
            fs = r.1;
        } 
        else if cmd == "create" {
            let r = do_create(disk, fs, command);
            disk = r.0;
            fs = r.1;
        } else if cmd == "create_dir" {
            let r = do_create_dir(disk, fs, command);
            disk = r.0;
            fs = r.1;
        }
        else if cmd == "remove" {
            let r = do_remove(disk, fs, command);
            disk = r.0;
            fs = r.1;
        } else if cmd == "stat" {
            let r = do_stat(disk, fs, command);
            disk = r.0;
            fs = r.1;
        } else if cmd == "copyin" {
            let r = do_copyin(disk, fs, command);
            disk = r.0;
            fs = r.1;
        } else if cmd == "copyout" {
            let r = do_copyout(disk, fs, command);
            disk = r.0;
            fs = r.1;
        } else if cmd == "cat" {
            let f = do_cat(fs, command);
            fs = f;
        } else if cmd == "truncate" {
            let f = do_truncate(fs, command);
            fs = f;
        }
        else if cmd == "exit" || cmd == "quit" {
            break;
        } else {
            println!("Unknown command: {}", line);
            println!("Type 'help' for a list of commands");
        }
    }
}

fn read_command() -> String {
    println!();
    print!("tfs> ");
    let _ = stdout().flush();
    let mut line = String::new();
    let bytes_read = stdin().read_line(&mut line).unwrap();
    if let Some('\n') = line.chars().next_back() {
        line.pop();
    }
    if let Some('\r') = line.chars().next_back() {
        line.pop();
    }
    println!();
    line
}

fn parse_command <'a>(command: &'a str) -> Vec<&str> {
    let mut v = Vec::new();
    let v2: Vec<&str> = command.split_whitespace().collect();
    for c in v2.iter() {
        if c != &" " {
            v.push(*c);
        }
    }
    v
}


fn do_help() {
    println!("Commands are:");
    println!("      format");
    println!("      mount");
    println!("      debug");
    println!("      create");
    println!("      create_dir");
    println!("      remove  <inode>");
    println!("      cat     <inode>");
    println!("      stat    <inode>");
    println!("      copyin  <inode> <file>");
    println!("      copyout <inode> <file>");
    println!("      truncate <inode> <byte_offset>");
    println!("      help");
    println!("      quite");
    println!("      exit");
}

fn do_format<'a>(mut disk: Disk<'a>, args: Vec<&str>) -> Disk<'a> {
    if args.len() != 1 {
        println!("Usage: format");
    } else {
        if FileSystem::format(&mut disk) {
            println!("disk formated.");
        } else {
            println!("format failed!");
        }
    }
    disk
}

fn do_mount<'a>(mut disk: Disk<'a>, mut fs: FileSystem<'a>,  args: Vec<&str>) -> (Disk<'a>, FileSystem<'a>) {
    if args.len() != 1 {
        println!("Usage: mount");
    } else {
        if fs.mount(&mut disk) {
            println!("disk mounted.");
        } else {
            println!("mount failed!");
        }
    }

    (disk, fs)
}

fn do_fuse_mount<'a>(mut disk: Disk<'a>, mut fs: FileSystem<'a>,  args: Vec<&str>) -> (Disk<'a>, FileSystem<'a>) {
    if args.len() != 1 {
        println!("Usage: mount");
    } else {
        println!("disk mounted.");
        DuffFS::new(FileSystem::from_disk(&mut disk)).mount();
    }

    (disk, fs)
}

fn do_create<'a>(mut disk: Disk<'a>, 
    mut fs: FileSystem<'a>,  args: Vec<&str>) 
    -> (Disk<'a>, FileSystem<'a>) {
        if args.len() != 1 {
            println!("Usage: create");
        } else {
            let fss = &mut fs as *mut FileSystem;
            let inumber = unsafe{(*fss).create()};
            if inumber >= 0 {
                println!("created inode {}", inumber);
            } else {
                println!("create failed!");
            }
        }
    
        (disk, fs) 
}

fn do_create_dir<'a>(mut disk: Disk<'a>, 
    mut fs: FileSystem<'a>,  args: Vec<&str>) 
    -> (Disk<'a>, FileSystem<'a>) {
        if args.len() != 1 {
            println!("Usage: create");
        } else {
            let fss = &mut fs as *mut FileSystem;
            let inumber = unsafe{(*fss).create_dir()};
            if inumber >= 0 {
                println!("created inode {}", inumber);
            } else {
                println!("create failed!");
            }
        }
    
        (disk, fs) 
}

fn do_remove<'a>(mut disk: Disk<'a>, 
    mut fs: FileSystem<'a>,  args: Vec<&str>) 
    -> (Disk<'a>, FileSystem<'a>) {
        if args.len() != 2 {
            println!("Usage: remove <inode>");
        } else {
            let inumber: usize = args[1].parse().unwrap();
            if fs.remove(inumber) {
                println!("removed inode {}", inumber);
            } else {
                println!("remove failed!");
            }
        }
    
        (disk, fs)
    
}

fn do_stat<'a>(mut disk: Disk<'a>, 
    mut fs: FileSystem<'a>,  args: Vec<&str>) 
    -> (Disk<'a>, FileSystem<'a>) {
        if args.len() != 2 {
            println!("Usage: stat <inode>");
        } else {
            let inumber: usize = args[1].parse().unwrap();
            let bytes = fs.stat(inumber);
            if bytes >= 0 {
                println!(" inode {} has size {} bytes", inumber, bytes);
            } else {
                println!("stat failed!");
            }
        }
    
        (disk, fs)
    
}

fn do_copyin<'a>(mut disk: Disk<'a>, 
    mut fs: FileSystem<'a>,  args: Vec<&str>) 
    -> (Disk<'a>, FileSystem<'a>) {
    if args.len() != 3 {
        println!("Usage: copyin <inode> <file>");
        return (disk, fs)
    } else {
        let inumber: usize = args[1].parse().unwrap();
        let (f, copied) = copyin(fs, args[2], inumber);
        // let fs, copied = r;
        if !copied {
            println!("copyin failed!");
        }
        (disk, f)
    }

}

fn do_copyout<'a>(mut disk: Disk<'a>, 
    mut fs: FileSystem<'a>,  args: Vec<&str>) 
    -> (Disk<'a>, FileSystem<'a>) {
    if args.len() != 3 {
        println!("Usage: copyout <inode> <file>");
        return (disk, fs)
    } else {
        let inumber: usize = args[1].parse().unwrap();
        let (f, copied) = copyout(fs, args[2], inumber);
        if !copied {
            println!("copyout failed!");
        }
        (disk, f)
    }

}



fn do_debug<'a>(mut disk: Disk<'a>, args: Vec<&str>) -> Disk<'a> {
    if args.len() != 1 {
        println!("Usage: debug");
    } else {
        FileSystem::debug(&mut disk);
    }
    return disk;
}

fn copyin<'a>(mut fs: FileSystem<'a>, path: &str, inumber: usize) -> (FileSystem<'a>, bool) {
    use std::fs::{File, OpenOptions};
    let file = OpenOptions::new().read(true).open(path);
    let mut file = match file {
         Ok(f) => f,
        _ => {
            println!("Unable to open {}", path);
            return (fs, false);
        }
    };

    let mut buffer = [0; BUFFER_SIZE];
    let mut offset = 0;
    let mut len = 0;
    let mut n = 0;
    loop {
        let result = match file.read(&mut buffer) {
            Ok(r) => r,
            _ => {
                break;
            }
        };
        if result <= 0 {
            break;
        }

        let actual = fs.write(inumber, &mut buffer, result, offset);
        if actual < 0 {
            println!("fs.write returned invalid result {}", actual);
            break;
        }
        len += result;
        n += 1;
        // file.seek(SeekFrom::Start( n as u64 * Disk::BLOCK_SIZE as u64));
        if actual as usize != result {
            println!("fs.write only wrote {} bytes, not {} bytes", actual, result);
            break;
        }
    }
    println!("{} bytes copied", len);
    (fs, true)
}


fn copyout<'a>(mut fs: FileSystem<'a>, path: &str, inumber: usize) -> (FileSystem<'a>, bool) {
    use std::fs::{File, OpenOptions};
    let file = OpenOptions::new().write(true).create(true).open(path);
    let mut file = match file {
         Ok(f) => f,
        _ => {
            println!("Unable to open {}", path);
            return (fs, false);
        }
    };

    let mut buffer = [0; BUFFER_SIZE];
    let mut offset = 0;

    loop {
        let result = fs.read(inumber, &mut buffer, BUFFER_SIZE, offset);
        if result <= 0 {
            break;
        }

        if result < BUFFER_SIZE as i64 {
            let mut buff = &buffer[0..result as usize];
            file.write(&mut buff);            
        } else {
            file.write(&mut buffer);
        }
        offset += result as usize;
    }
    println!("{} bytes copied", offset);
    (fs, true)
}

fn do_cat<'a>(mut fs: FileSystem<'a>, args: Vec<&str>) -> FileSystem<'a> {
    if args.len() != 2 {
        println!("Usage: cat <inode>");
        return fs;
    } else {
        let inumber = args[1].parse().unwrap();
        let (f, copied) = copyout(fs, "/dev/stdout", inumber);
        if !copied {
            println!("cat failed!");
        }
        return f;
    }
}

fn do_truncate<'a>(mut fs: FileSystem<'a>, args: Vec<&str>) -> FileSystem<'a> {
    if args.len() != 3 {
        println!("Usage: truncate <inode> <byte_offset>");
        return fs;
    } else {
        let inumber = args[1].parse().unwrap();
        let offset = args[2].parse().unwrap();
        // let (f, copied) = copyout(fs, "/dev/stdout", inumber);
        fs.truncate(inumber, offset);
        return fs;
    }
}