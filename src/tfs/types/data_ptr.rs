
use crate::tfs::disk::Disk;
use crate::tfs::constants::*;
use super::Block;
use crate::tfs::utility::*;

pub struct InodeDataPointer<'d> {
    pub direct_ptrs: Vec<u32>,
    pub indirect_ptrs: Vec<Vec<u32>>,    // the len of indirect_ptrs = depth
    disk: Disk<'d>,
    pub root_ptr: u32
}

impl<'d> InodeDataPointer<'d> {
    pub fn new(disk: Disk<'d>, root_ptr: u32) -> Self {
        InodeDataPointer {
            root_ptr,
            disk,
            direct_ptrs: Vec::new(),
            indirect_ptrs: Vec::new()
        }
    }

    pub fn with_depth(depth: usize, disk: Disk<'d>, root_ptr: u32) -> Self {
        let mut d = Self::new(disk, root_ptr);
        if root_ptr > 0 {
            d.set_depth(depth)
        } else {
            d
        }
    }

    pub fn debug(&self) {
        println!("*********** Inode Data Pointer *************");
        println!("Root Ptr: {}", self.root_ptr);
        println!("Direct Ptrs: {:?}", self.direct_ptrs.len());
        println!("InDirect Ptrs: {:?}", self.indirect_ptrs.len());
        println!("*********** End *************");
    }

    pub fn to_depth(mut self, n: usize) -> Self {
        let data_block = self.root_ptr;
        let index_zero = |array: [u32; POINTERS_PER_BLOCK]| {
            let mut i = 0;
            loop {
                if i == array.len() || array[i] == 0 {
                    break
                }
                i += 1;
            }
            i
        };
        match n {
            0 => {  // direct pointers
                let mut block = Block::new();
                self.disk.read(self.root_ptr as usize, block.data_as_mut());
                let i = index_zero(block.pointers());
                self.direct_ptrs = Vec::from(&block.pointers()[0..i]);
            },
            _ => { // indirect pointers
                let mut data_blks = Block::new();
                let mut data_blocks = Vec::new();
                for blk in self.direct_ptrs.iter() {
                    if *blk == 0 {
                        break
                    }
                    self.disk.read(*blk as usize, data_blks.data_as_mut());
                    let i = index_zero(data_blks.pointers());
                    data_blocks.extend_from_slice(&data_blks.pointers()[..i]);
                }
                self.direct_ptrs = data_blocks;
            }
        }
        //
        self
    }

    pub fn set_depth(mut self, level: usize) -> Self {
        for n in 0..level+1 {
            self = self.to_depth(n);
            if n < level {
                let mut start = 0;
                let mut end = POINTERS_PER_BLOCK;
                let mut ptrs = Vec::new();
                loop {
                    if end >= self.direct_pointers().len() {
                        end = self.direct_pointers().len();
                        ptrs.extend_from_slice(&self.direct_pointers()[start..end]);
                        break
                    }
                    ptrs.extend_from_slice(&self.direct_pointers()[start..end]);
                    start = end;
                    end = start + POINTERS_PER_BLOCK;
                }
                if ptrs.len() > 0 {
                    self.indirect_ptrs.push(ptrs);
                }
            }
        }
        self.indirect_ptrs.reverse();
        self
    }

    pub fn blocks(self) -> Vec<u32> {
        self.direct_ptrs
    }

    pub fn direct_pointers(&self) -> &Vec<u32> {
        &self.direct_ptrs
    }

    pub fn depth(&self) -> usize {
        ((self.direct_ptrs.len() as f64).log(POINTERS_PER_BLOCK as f64)).floor() as usize
    }

    pub fn add_data_block(&mut self, blk: i64) -> i64 {
        if blk > 0 {
            let mut p = self.direct_ptrs.len();
            let ptrs = p;
            let depth_pos = self.compute_positions(None);
           
            if self.indirect_ptrs.len() < depth_pos.len() {
                // increase depth / level
                self.indirect_ptrs.push(vec![blk as u32]);
                return (self.indirect_ptrs.len() + 1) as i64;
            }

            let mut d = 0;
            let n = depth_pos.len();
            loop {
                if d < n {
                    let i = n - d - 1;
                    let k = ptrs % POINTERS_PER_BLOCK;
                    if k == 0 && self.indirect_ptrs[i].len() == depth_pos[d] {
                        self.indirect_ptrs[i].push(blk as u32);
                        return (i + 1) as i64;
                    }
                    d += 1;
                } else {
                    break;
                }
            }
            self.direct_ptrs.push(blk as u32);
            return 0;
        } else {
            return -1;
        }
    }

    fn compute_positions(&self, len: Option<usize>) -> Vec<usize> {
        let mut p = match len {
            Some(l) => l,
            None => self.direct_ptrs.len()
        };
        let ptrs = p;

        // get highest depth
        let mut depth = self.depth();
            
        // get the positions at the different depths
        let mut depth_pos = vec![];
        loop {
            if depth > 0 {
                p = (ptrs as f64 / POINTERS_PER_BLOCK.pow(depth as u32) as f64).floor() as usize;
                depth_pos.push(p);
                depth -= 1;
            } else {
                break
            }
        }

        depth_pos
    }

    pub fn truncate(&mut self, byte_offset: usize) -> i64 {
        // compute direct pointer offset
        let direct_offset = (byte_offset as f64 / Disk::BLOCK_SIZE as f64).ceil() as usize;
        let mut depth_pos = self.compute_positions(Some(direct_offset));
        depth_pos.reverse();

        let mut free_blocks = vec![];

        println!("bb: {}, {}", byte_offset, direct_offset);

        if direct_offset < self.direct_ptrs.len() { // carry out truncate
            for i in direct_offset..self.direct_ptrs.len() {
                free_blocks.push(self.direct_ptrs[i]);
            }
            self.direct_ptrs.truncate(direct_offset);

            for i in 0..depth_pos.len() {
                if depth_pos[i] < self.indirect_ptrs[i].len() - 1 {
                    for j in (depth_pos[i]+1)..self.indirect_ptrs[i].len() {
                        free_blocks.push(
                            self.indirect_ptrs[i][j]
                        );
                    }
                    self.indirect_ptrs[i].truncate(depth_pos[i]+1);
                } else {
                    break;
                }
            }
            return -1;  // truncation was successfull
        } else if direct_offset > self.direct_ptrs.len() {
            return (direct_offset - self.direct_ptrs.len()) as i64; // number of blocks to add for padding
        } else {
            return 0;   // nothing to truncate
        }
    }

    pub fn save(&mut self) {
        let this = self as *mut Self;
        let fill_zeros = |array: &[u32]| {
            let mut i = 0;
            let mut block = Block::new();
            loop {
                if i == array.len(){
                    break
                }
                block.pointers_as_mut()[i] = array[i];
                i += 1;
            }
            return block.data();
        };

        let write_disk = |prev_ptrs: &mut Vec<u32>, curr_ptrs: &mut Vec<u32>, mut disk: Disk| {
            let mut j = 0;
                let mut start = 0;
                let mut end = POINTERS_PER_BLOCK;
                loop {
                    if end > curr_ptrs.len() {
                        end = curr_ptrs.len();
                        disk.write(prev_ptrs[j] as usize, &mut fill_zeros(&curr_ptrs[start..end]));
                        break;
                    } else {
                        disk.write(prev_ptrs[j] as usize, &mut fill_zeros(&curr_ptrs[start..end]));
                        start = end;
                        end = start + POINTERS_PER_BLOCK;
                        j += 1;
                    }

                    if j == prev_ptrs.len() {
                        break;
                    }
                }
        };

        self.indirect_ptrs.reverse();   // temporarily reverse the list

        for i in 0..self.indirect_ptrs.len() {
            unsafe {
                if i == 0 {
                    self.disk.write(self.root_ptr as usize, &mut fill_zeros(self.indirect_ptrs[0].as_slice()));
                } else {
                    write_disk(&mut (*this).indirect_ptrs[i - 1], &mut self.indirect_ptrs[i], self.disk.clone());
                }
                if i + 1 == self.indirect_ptrs.len() {
                    write_disk(&mut (*this).indirect_ptrs[i], &mut self.direct_ptrs, self.disk.clone());
                }
            }
        }

        if self.indirect_ptrs.len() == 0 {
            let mut tmp = &mut fill_zeros(self.direct_ptrs.as_slice());            
            self.disk.write(self.root_ptr as usize, tmp);
        }

        self.indirect_ptrs.reverse(); // reverse to what it was before... 

        // println!("Details saved: depth: {}, indirect -> {}, direct -> {}", self.depth(), self.indirect_ptrs.len(), self.direct_ptrs.len());
    }

}