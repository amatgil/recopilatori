use crate::{
    existeix, inform, oopsie, recurse_files, report, DirEntry, ANSICLEAR, ANSIGREEN, ANSIITALIC,
    ANSIYELLOW,
};
use sqlx::sqlite::SqlitePool;

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
        if matches.is_empty() {
            report(&format!(
                "{}\t{}NOU{}",
                file.path().display(),
                ANSIGREEN,
                ANSICLEAR
            ));
        } else {
            report(&format!(
                "{}\t{}DUPLICAT{}\t[{}]",
                file.path().display(),
                ANSIYELLOW,
                ANSICLEAR,
                matches.join(", ")
            ));
        }
        let end_time = Instant::now();

        inform(&format!(
            "Checking existance of {} took '{}{:#?}{}'\n",
            file.file_name().to_string_lossy(),
            ANSIITALIC,
            end_time - start_time,
            ANSICLEAR
        ));
    }
    Ok(())
}

pub async fn existance_check(pool: SqlitePool, folder: String) -> Result<(), sqlx::Error> {
    let (tx, rx) = mpsc::channel();

    let checker_handle = tokio::spawn(file_bulk_exists_check(pool, rx)).await;
    let reader_handle = thread::spawn(move || {
        for file in recurse_files(Path::new(&folder))? {
            tx.send(file).unwrap_or_else(|e| {
                oopsie(&format!("Error sending to file reading thread: {e}"), 11)
            });
        }
        Ok::<(), sqlx::Error>(())
    });

    match checker_handle {
        Ok(c) => c?,
        Err(e) => oopsie(&format!("Error comprovant si el fitxer existi: '{e}'"), 11),
    };

    match reader_handle.join() {
        Ok(r) => r?,
        Err(_) => oopsie("Error llegint fitxers!", 1),
    };

    Ok(())
}
