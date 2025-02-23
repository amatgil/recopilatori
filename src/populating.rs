use crate::{
    hashes_of, inform, insert_file, log, mark_not_seen_as_deleted, oopsie, recurse_files,
    ANSICLEAR, ANSIITALIC, MAX_ALLOWED_OPEN_FILE_COUNT,
};
use regex::Regex;
use std::{
    fs::{self, DirEntry},
    path::Path,
    sync::mpsc::{self, Receiver, RecvError},
};

use sqlx::{
    sqlite::SqlitePool,
    types::chrono::{DateTime, Utc},
};

async fn insert_file_report(
    pool: SqlitePool,
    file: DirEntry,
    ignore_patterns: &[Regex],
    folder: &str,
) -> Result<(), sqlx::Error> {
    let curr_time: DateTime<Utc> = Utc::now();

    let real_path = file.path();
    let db_path = file.path().clone();
    let db_path = db_path.strip_prefix(&folder).unwrap_or_else(|_| {
        oopsie(
            "Error intern: fitxer de la carpeta no està dins de la carpeta?",
            1,
        )
    });

    if let Some(r) = ignore_patterns
        .iter()
        .find(|r| r.is_match(&db_path.display().to_string()))
    {
        inform(&format!(
            "Ignoring file '{}' (per regex '{}')",
            db_path.display(),
            r
        ));
        return Ok(());
    }
    let file_contents: Vec<u8> = fs::read(&real_path)?;

    inform(&format!("Buscant la hash de: {db_path:?}"));
    let (short_hash, full_hash) = hashes_of(&file_contents);

    insert_file(
        &pool,
        &real_path,
        db_path,
        short_hash,
        full_hash,
        file_contents.len() as i64,
        curr_time,
    )
    .await?;

    let delta = Utc::now() - curr_time;
    inform(&format!(
        "Processing file {} took '{ANSIITALIC}{delta}{ANSICLEAR}'\n",
        db_path.display()
    ));

    Ok(())
}

/// Make database reflect state of `folder`
pub async fn populate(
    pool: SqlitePool,
    folder: String,
    ignore_patterns: Vec<Regex>,
) -> Result<(), sqlx::Error> {
    let start_time: DateTime<Utc> = Utc::now();
    let (tx, rx) = mpsc::sync_channel(MAX_ALLOWED_OPEN_FILE_COUNT);

    inform("Starting populate...");

    let folder_b = folder.clone();
    let bulk_insertion_handle = tokio::task::spawn_blocking(move || {
        bulk_insert_files(pool, rx, folder_b, ignore_patterns, start_time)
    });
    inform("Bulk insertion thread is online");

    let reader_handle = tokio::spawn(async move { recurse_files(Path::new(&folder), &tx) });
    inform("File reading thread is online");

    match reader_handle.await {
        Ok(r) => r?,
        Err(e) => oopsie(&format!("Error llegint fitxers: '{e}'"), 2),
    };
    match bulk_insertion_handle.await {
        Ok(h) => h.await?,
        Err(e) => oopsie(&format!("Error fent les insercions: '{e}"), 2),
    }

    inform("Finished populating database successfully!");

    Ok(())
}

async fn bulk_insert_files(
    pool: SqlitePool,
    rx: Receiver<DirEntry>,
    folder: String,
    ignore_patterns: Vec<Regex>,
    start_time: DateTime<Utc>,
) -> Result<(), sqlx::Error> {
    loop {
        match rx.recv() {
            Ok(file) => {
                log(&format!("Received file {}", file.path().display()));
                insert_file_report(pool.clone(), file, &ignore_patterns, &folder).await?;
            }
            Err(RecvError) => break,
        }
    }

    mark_not_seen_as_deleted(pool.clone(), &start_time).await?;
    Ok::<(), sqlx::Error>(())
}
