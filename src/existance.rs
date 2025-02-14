use crate::*;
use sqlx::sqlite::*;

use std::{
    path::Path,
    sync::mpsc::{self, Receiver},
    thread,
    time::Instant,
};

async fn file_bulk_exists_check(
    pool: SqlitePool,
    rx: Receiver<DirEntry>,
) -> Result<(), sqlx::Error> {
    while let Ok(file) = rx.recv() {
        let start_time = Instant::now();

        let matches = existeix(&pool, &file.path()).await?;
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
            "Checking existance of {} took '{ANSIITALIC}{:#?}{ANSICLEAR}'\n",
            file.file_name().to_string_lossy(),
            end_time - start_time
        ));
    }
    Ok(())
}

pub async fn existance_check(pool: SqlitePool, folder: String) -> Result<(), sqlx::Error> {
    let (tx, rx) = mpsc::channel();

    let reader_handle = thread::spawn(move || {
        for file in recurse_files(Path::new(&folder))? {
            tx.send(file).unwrap_or_else(|e| {
                error(&format!("Error sending to hashing thread: {e}"));
                std::process::exit(1);
            })
        }
        Ok::<(), sqlx::Error>(())
    });

    let checker_handle = tokio::spawn(file_bulk_exists_check(pool, rx)).await;

    match checker_handle {
        Ok(c) => c?,
        Err(e) => {
            error(&format!("Error comprovant si el fitxer existi: '{e}'"));
            std::process::exit(2);
        }
    };

    match reader_handle.join() {
        Ok(r) => r?,
        Err(_) => {
            error(&format!("Error llegint fitxers!"));
            std::process::exit(2);
        }
    };

    Ok(())
}
