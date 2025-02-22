pub mod databaseing;
pub mod existance;
pub mod geoloc;
pub mod populating;
pub use databaseing::*;
use regex::Regex;

use std::collections::VecDeque;
use std::fs;
use std::fs::DirEntry;
use std::io;
use std::path::Path;
use std::sync::{Arc, Mutex};
use std::thread::sleep;
use std::time::{Duration, Instant};

pub const ANSIRED: &str = "\x1b[1;31m";
pub const ANSIGREEN: &str = "\x1b[1;32m";
pub const ANSIYELLOW: &str = "\x1b[1;33m";
pub const ANSIBLUE: &str = "\x1b[1;34m";
pub const ANSIITALIC: &str = "\x1b[3m";
pub const ANSICLEAR: &str = "\x1b[0m";

pub const MAX_ALLOWED_OPEN_FILE_COUNT: usize = 1_000_000;

/// 'dir' should be a directory, otherwise an empty vec will be returned
pub fn recurse_files(dir: &Path, queue: Arc<Mutex<VecDeque<DirEntry>>>) -> io::Result<()> {
    if dir.is_dir() {
        for entry in fs::read_dir(dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.is_dir() {
                recurse_files(&path, queue.clone())?;
            } else {
                let mut q = queue.lock().unwrap();
                while q.len() >= MAX_ALLOWED_OPEN_FILE_COUNT {
                    drop(q);
                    sleep(Duration::from_micros(100));
                    q = queue.lock().unwrap();
                }
                q.push_back(entry);
            }
        }
    }
    Ok(())
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

#[must_use]
pub fn short_hash_of(file_contents: &[u8]) -> [u8; 16] {
    const SHORT_SIZE: usize = 1_000_000; // 1MB
    md5::compute(&file_contents[0..SHORT_SIZE.min(file_contents.len())])
}

#[must_use]
pub fn full_hash_of(file_contents: &[u8]) -> [u8; 16] {
    md5::compute(file_contents)
}

#[must_use]
pub fn hashes_of(full_data: &[u8]) -> ([u8; 16], [u8; 16]) {
    let start_hash = Instant::now();
    let h = (short_hash_of(full_data), full_hash_of(full_data));
    let end_hash = Instant::now();

    inform(&format!(
        "Hash trobada, tardant: '{:?}'",
        end_hash - start_hash
    ));
    h
}

pub fn oopsie(s: &str, code: i32) -> ! {
    error(s);
    std::process::exit(code);
}

pub fn get_ignore_patterns() -> Result<Vec<Regex>, sqlx::Error> {
    let ignore_patterns: Vec<Regex> = match fs::read_to_string("recopilatori.ignored") {
        Ok(c) => {
            let r = c
                .split('\n')
                .filter(|s| !s.is_empty())
                .map(Regex::new)
                .collect::<Result<Vec<Regex>, _>>()
                .unwrap_or_else(|e| {
                    oopsie(
                        &format!("ERROR: regex invàlida al fitxer d'ignorats: '{e}'",),
                        1,
                    )
                });

            inform(&format!(
                "recopilatori.ignored detectat amb '{}' patrons\n",
                r.len()
            ));
            r
        }
        Err(e) if e.kind() == io::ErrorKind::NotFound => {
            inform("No `recopilatori.ignored` detected\n");
            vec![]
        }
        e => {
            e?;
            unreachable!()
        }
    };
    Ok(ignore_patterns)
}
