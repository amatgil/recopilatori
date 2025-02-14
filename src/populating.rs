use crate::*;
use regex::Regex;
use std::{
    fs, io,
    path::{Path, PathBuf},
    process,
    sync::mpsc,
    thread,
    time::Instant,
};

use sqlx::{
    sqlite::*,
    types::chrono::{DateTime, Utc},
};

/// Make database reflect state of `folder`
pub async fn populate(
    pool: SqlitePool,
    folder: String,
    ignore_patterns: Vec<Regex>,
) -> Result<(), sqlx::Error> {
    let start_time: DateTime<Utc> = Utc::now();
    let (tx, rx) = mpsc::channel();

    async fn hash_files(
        pool: SqlitePool,
        rx: mpsc::Receiver<fs::DirEntry>,
        folder: String,
        ignore_patterns: Vec<Regex>,
        start_time: DateTime<Utc>,
    ) -> Result<(), sqlx::Error> {
        while let Ok(file) = rx.recv() {
            let curr_time: DateTime<Utc> = Utc::now();

            let real_path = file.path();
            let db_path = file.path().to_owned();
            let db_path = db_path.strip_prefix(&folder).unwrap_or_else(|_| {
                error("Error intern: fitxer de la carpeta no està dins de la carpeta?");
                process::exit(1)
            });

            if let Some(r) = ignore_patterns
                .iter()
                .filter(|r| r.is_match(&db_path.display().to_string()))
                .next()
            {
                inform(&format!(
                    "Ignoring file '{}' (per regex '{}')",
                    db_path.display(),
                    r
                ));
                continue;
            }
            let file_contents: Vec<u8> = fs::read(&real_path)?;

            inform(&format!("Tractant: {:?}", db_path));

            let start_hash = Instant::now();
            let (short_hash, full_hash) = hashes_of(&file_contents);
            let end_hash = Instant::now();
            inform(&format!(
                "Hash trobada, tardant: '{:?}'",
                end_hash - start_hash
            ));

            inform("Insertant a BD...");
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
                "Processing file {} took '{ANSIITALIC}{delta}{ANSICLEAR}'",
                db_path.display()
            ));

            eprintln!();

            inform("Marking those not seen as deleted...");
            mark_not_seen_as_deleted(pool.clone(), &start_time).await?;
            inform("Finished marking those not seen as deleted");
        }
        Ok::<(), sqlx::Error>(())
    }

    let hasher_handle = tokio::spawn(hash_files(
        pool,
        rx,
        folder.clone(),
        ignore_patterns,
        start_time,
    ))
    .await;

    let reader_handle = thread::spawn(move || {
        for file in recurse_files(Path::new(&folder))? {
            match tx.send(file) {
                Ok(()) => {}
                Err(e) => {
                    error(&format!("Error sending to hashing thread: {e}"));
                    std::process::exit(1);
                }
            };
        }
        Ok::<(), sqlx::Error>(())
    });

    match reader_handle.join() {
        Ok(r) => r?,
        Err(_) => {
            error(&format!("Error llegint fitxers!"));
            std::process::exit(2);
        }
    };
    match hasher_handle {
        Ok(h) => h?,
        Err(e) => {
            error(&format!("Error fent hash!: {e}"));
            std::process::exit(2);
        }
    }

    Ok(())
}
