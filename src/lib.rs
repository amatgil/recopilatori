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
    eprintln!("INFO: {s}");
}

pub fn report(s: &str) {
    println!("RESULTAT: {s}");
}

pub fn short_hash_of(file_contents: &[u8]) -> [u8; 16] {
    const SHORT_SIZE: usize = 1000000; // 1MB
    md5::compute(&file_contents[0..SHORT_SIZE.min(file_contents.len())])
}
pub fn full_hash_of(file_contents: &[u8]) -> [u8; 16] {
    md5::compute(file_contents)
}

pub fn hashes_of(p: &PathBuf) -> io::Result<([u8; 16], [u8; 16])> {
    let full_data: Vec<u8> = fs::read(p)?;
    Ok((short_hash_of(&full_data), full_hash_of(&full_data)))
}
