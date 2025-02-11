pub mod databaseing;
pub mod geoloc;
pub use databaseing::*;

use md5;
use std::fs;
use std::fs::DirEntry;
use std::io;
use std::path::Path;

pub const ANSIRED: &'static str = "\x1b[1;31m";
pub const ANSIGREEN: &'static str = "\x1b[1;32m";
pub const ANSIYELLOW: &'static str = "\x1b[1;33m";
pub const ANSIBLUE: &'static str = "\x1b[1;34m";
pub const ANSIITALIC: &'static str = "\x1b[3m";
pub const ANSICLEAR: &'static str = "\x1b[0m";

/// 'dir' should be a directory, otherwise an empty vec will be returned
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
    eprintln!("{ANSIBLUE}INFO:{ANSICLEAR}\t{s}");
}

pub fn report(s: &str) {
    println!("{ANSIGREEN}OUTPUT:{ANSICLEAR}\t{s}");
}

pub fn error(s: &str) {
    eprintln!("{ANSIRED}ERROR:{ANSICLEAR}\t{s}");
}

pub fn short_hash_of(file_contents: &[u8]) -> [u8; 16] {
    const SHORT_SIZE: usize = 1000000; // 1MB
    md5::compute(&file_contents[0..SHORT_SIZE.min(file_contents.len())])
}
pub fn full_hash_of(file_contents: &[u8]) -> [u8; 16] {
    md5::compute(file_contents)
}

pub fn hashes_of(full_data: &[u8]) -> ([u8; 16], [u8; 16]) {
    (short_hash_of(&full_data), full_hash_of(&full_data))
}
