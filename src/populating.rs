use crate::{
    hashes_of, inform, insert_file, log, mark_not_seen_as_deleted, oopsie, recurse_files,
    ANSICLEAR, ANSIITALIC, MAX_ALLOWED_OPEN_FILE_COUNT,
    short_hash_of
};
use regex::Regex;
use std::{
    fs::{self, DirEntry},
    path::Path,
    sync::mpsc::{self, Receiver, RecvError},
    path::PathBuf
};
use std::sync::Arc;
use tokio::task::JoinSet;

use sqlx::{
    sqlite::SqlitePool,
    types::chrono::{DateTime, Utc},
};

async fn insert_file_report(
    pool: SqlitePool,
    file: DirEntry,
    ignore_patterns: Arc<Vec<Regex>>,
    folder: Arc<String>,
) -> Result<(), sqlx::Error> {
    let curr_time: DateTime<Utc> = Utc::now();

    let real_path = file.path();
    let db_path = file.path().clone();
    let db_path = db_path.strip_prefix(&*folder).unwrap_or_else(|_| {
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
    //let (short_hash, full_hash) = hashes_of(&file_contents);
    let short_hash = short_hash_of(&file_contents);

    inform(&format!("Starting insertion of file '{}'", db_path.display()));
    insert_file(
        &pool,
        &real_path,
        db_path,
        short_hash,
        None, // full_hash,
        file_contents.len() as i64,
        curr_time,
    )
    .await?;

    let delta = Utc::now() - curr_time;
    inform(&format!(
        "Finished processsing file '{}', took '{ANSIITALIC}{delta}{ANSICLEAR}'\n",
        db_path.display()
    ));

    Ok(())
}

/// Make database reflect state of `folder`
pub async fn populate(
    pool: SqlitePool,
    folder: Arc<String>,
    ignore_patterns: Arc<Vec<Regex>>,
) -> Result<(), sqlx::Error> {
    let start_time: DateTime<Utc> = Utc::now();
    let (tx, rx) = mpsc::sync_channel(MAX_ALLOWED_OPEN_FILE_COUNT);

    inform("Starting populate...");

    let folder_b = folder.clone();
    let bulk_insertion_handle = tokio::spawn(async move {
        bulk_insert_files(pool, rx, folder_b, ignore_patterns.clone(), start_time).await
    });
    inform("Bulk insertion thread is online");

    let reader_handle = tokio::spawn(async move {
        recurse_files(PathBuf::from((*folder).clone()), tx).await
    });
    inform("File reading thread is online");

    let (bulk_insert_ret, reader_ret) = tokio::join!(
        bulk_insertion_handle, reader_handle
    );
    
    match bulk_insert_ret {
        Ok(h) => h?,
        Err(e) => oopsie(&format!("Error fent les insercions: '{e}"), 2),
    }
    match reader_ret {
        Ok(r) => r?,
        Err(e) => oopsie(&format!("Error llegint fitxers: '{e}'"), 2),
    };

    inform("Finished populating database successfully!");

    Ok(())
}

async fn bulk_insert_files(
    pool: SqlitePool,
    rx: Receiver<DirEntry>,
    folder: Arc<String>,
    ignore_patterns: Arc<Vec<Regex>>,
    start_time: DateTime<Utc>,
) -> Result<(), sqlx::Error> {
    const MAX_CONCURRENT: usize = 3;

    let mut join_set = JoinSet::new();
    loop {
        log("Waiting for file");
        
        // Too many are open, wait until one completes
        while join_set.len() >= MAX_CONCURRENT {
            log("Over six threads exist, executing one to completion");
            join_set.join_next().await.unwrap().unwrap()?;
        }

        match rx.recv() {
            Ok(file) => {
                log(&format!("Received file {}", file.path().display()));
                let _ = join_set.spawn(insert_file_report(pool.clone(), file, ignore_patterns.clone(), folder.clone()));
            }
            Err(RecvError) => break,
        }
    }

    mark_not_seen_as_deleted(pool.clone(), &start_time).await?;
    Ok::<(), sqlx::Error>(())
}
