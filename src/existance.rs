use crate::*;
use sqlx::{
    sqlite::*,
    types::chrono::{DateTime, Utc},
};

use std::{
    fs, io,
    path::{Path, PathBuf},
    process,
    sync::mpsc,
    thread,
    time::Instant,
};

pub async fn existance_check(pool: &SqlitePool, folder: &str) -> Result<(), sqlx::Error> {
    for file in recurse_files(&Path::new(&folder))? {
        let start_time = Instant::now();
        let matches = existeix(pool, &file.path()).await?;
        if matches.len() > 0 {
            report(&format!(
                "{}\t{ANSIYELLOW}DUPLICAT{ANSICLEAR}\t[{}]",
                file.path().display(),
                matches.join(", ")
            ));
        } else {
            report(&format!(
                "{}\t{ANSIGREEN}NOU{ANSICLEAR}",
                file.path().display()
            ));
        }
        let end_time = Instant::now();
        inform(&format!(
            "Checking existance of {} took '{ANSIITALIC}{:#?}{ANSICLEAR}'",
            file.file_name().to_string_lossy(),
            end_time - start_time
        ));

        eprintln!();
    }
    Ok(())
}
