use crate::{
    existeix, inform, oopsie, recurse_files, report, DirEntry, ANSICLEAR, ANSIGREEN, ANSIITALIC,
    ANSIYELLOW, MAX_ALLOWED_OPEN_FILE_COUNT,
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
    queue: Receiver<DirEntry>,
) -> Result<(), sqlx::Error> {
    while let Ok(file) = queue.recv() {
        report_existance(&pool, file).await?;
    }
    Ok(())
}

async fn report_existance(
    pool: &sqlx::Pool<sqlx::Sqlite>,
    file: DirEntry,
) -> Result<(), sqlx::Error> {
    let start_time = Instant::now();
    let matches = existeix(pool, &file.path()).await?;
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
    Ok(())
}

pub async fn existance_check(pool: SqlitePool, folder: String) -> Result<(), sqlx::Error> {
    let (tx, rx) = mpsc::sync_channel(MAX_ALLOWED_OPEN_FILE_COUNT);

    let checker_handle = thread::spawn(move || file_bulk_exists_check(pool, rx));
    inform("Querying thread up and running");

    let reader_handle = thread::spawn(move || {
        recurse_files(Path::new(&folder), &tx)?;
        Ok::<(), sqlx::Error>(())
    });
    inform("File-reading thread up and running");

    match checker_handle.join() {
        Ok(c) => c.await?,
        Err(_) => oopsie(&format!("Error comprovant si el fitxer existi"), 11),
    };

    match reader_handle.join() {
        Ok(r) => r?,
        Err(_) => oopsie("Error llegint fitxers!", 1),
    };

    Ok(())
}
