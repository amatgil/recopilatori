use recopilatori::*;
use std::{path::Path, time::Instant};

use sqlx::sqlite::*;

async fn populate() -> Result<(), sqlx::Error> {
    // Create a connection pool
    let pool = SqlitePoolOptions::new()
        .max_connections(5)
        .connect("sqlite://dades.db")
        .await?;

    setup(&pool).await?;

    let folder = "./fitxers_a_tractar";
    for file in recurse_files(Path::new(folder))? {
        let real_path = file.path();
        let db_path = file.path().to_owned();
        let db_path = db_path
            .strip_prefix(folder)
            .expect("Error intern: fitxer de la carpeta no està dins de la carpeta?");

        inform(&format!("Tractant: {:?}", db_path));

        let start = Instant::now();
        let (short_hash, full_hash) = hashes_of(&real_path)?;
        let end = Instant::now();
        inform(&format!("Hash trobada, tardant: '{:?}'", end - start));

        inform("Insertant a BD...");
        insert_file(&pool, &real_path, db_path, short_hash, full_hash);
    }
    Ok(())
}

#[tokio::main]
async fn main() -> Result<(), sqlx::Error> {
    populate().await
}
