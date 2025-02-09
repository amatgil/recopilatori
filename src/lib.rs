pub mod databaseing;
pub use databaseing::*;

use md5;
use std::fs;
use std::fs::DirEntry;
use std::io;
use std::path::{Path, PathBuf};

pub fn recurse_files(dir: &Path) -> io::Result<Vec<DirEntry>> {
    let mut r = vec![];
    if dir.is_dir() {
        for entry in fs::read_dir(dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.is_dir() {
                r.append(&mut recurse_files(&path)?);
            } else {
                r.push(entry);
            }
        }
    }
    Ok(r)
}

pub fn inform(s: &str) {
    println!("INFO: {s}");
}

pub fn hashes_of(p: &PathBuf) -> io::Result<([u8; 16], [u8; 16])> {
    const SHORT_SIZE: usize = 1000000; // 1MB
    let full_data: Vec<u8> = fs::read(p)?;
    let full_hash = md5::compute(&full_data);

    if full_data.len() <= SHORT_SIZE {
        Ok((full_hash, full_hash))
    } else {
        let short_hash = md5::compute(&full_data[0..SHORT_SIZE]);
        Ok((short_hash, full_hash))
    }
}
