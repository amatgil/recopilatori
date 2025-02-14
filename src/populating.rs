use crate::*;
use regex::Regex;
use std::{fs, path::Path, sync::mpsc, thread};

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

    async fn bulk_insert_files(
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
                oopsie(
                    "Error intern: fitxer de la carpeta no està dins de la carpeta?",
                    1,
                )
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

            inform(&format!("Buscant la hash de: {:?}", db_path));
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

            mark_not_seen_as_deleted(pool.clone(), &start_time).await?;
        }
        Ok::<(), sqlx::Error>(())
    }

    let bulk_insertion_handle = tokio::spawn(bulk_insert_files(
        pool,
        rx,
        folder.clone(),
        ignore_patterns,
        start_time,
    ))
    .await;

    let reader_handle = thread::spawn(move || {
        for file in recurse_files(Path::new(&folder))? {
            tx.send(file)
                .unwrap_or_else(|e| oopsie(&format!("Error sending to hashing thread: {e}"), 1));
        }
        Ok::<(), sqlx::Error>(())
    });

    match reader_handle.join() {
        Ok(r) => r?,
        Err(_) => oopsie("Error llegint fitxers!", 2),
    };

    match bulk_insertion_handle {
        Ok(h) => h?,
        Err(e) => oopsie(&format!("Error fent les insercions!: {e}"), 2),
    }

    Ok(())
}
