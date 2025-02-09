use crate::*;
use sqlx::*;

#[derive(FromRow, Debug, Clone)]
pub struct TipusFitxer {
    pub tipus_id: u32,
    pub tipus_nom: String,
}

pub async fn setup(pool: &SqlitePool) -> Result<(), sqlx::Error> {
    inform("Creating tables if they don't exist...");
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS tipus_fitxers (
            tipus_id INTEGER PRIMARY KEY,
            tipus_nom TEXT NOT NULL UNIQUE
        );"#,
    )
    .execute(pool)
    .await?;

    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS fitxers (
            fitxer_id INTEGER PRIMARY KEY,
            full_path TEXT NOT NULL UNIQUE,
            tipus_id INTEGER REFERENCES tipus_fitxers
        );"#,
    )
    .execute(pool)
    .await?;

    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS hashes (
            hash_id PRIMARY KEY,
            short_hash_1mb UUID NOT NULL,
            full_hash UUID NOT NULL
        );
        "#,
    )
    .execute(pool)
    .await?;

    Ok(())
}

pub async fn clear_all(pool: &SqlitePool) -> Result<(), sqlx::Error> {
    inform("Deleting all data...");
    for q in [
        "DELETE FROM hashes;",
        "DELETE FROM fitxers;",
        "DELETE FROM tipus_fitxers",
    ] {
        sqlx::query(q).execute(pool).await?;
    }
    Ok(())
}

pub async fn get_tipus_id_of(pool: &SqlitePool, path: &Path) -> Result<Option<i64>, sqlx::Error> {
    if let Some(ext) = path.extension() {
        let id = sqlx::query("INSERT OR IGNORE INTO tipus_fitxers (tipus_nom) VALUES (?)")
            .bind(ext.to_string_lossy())
            .execute(pool)
            .await?
            .last_insert_rowid();
        Ok(Some(id))
    } else {
        Ok(None)
    }
}
pub async fn insert_file(
    pool: &SqlitePool,
    real_path: &Path,
    db_path: &Path,
    short_hash: [u8; 16],
    full_hash: [u8; 16],
) -> Result<(), sqlx::Error> {
    let tipus_id = get_tipus_id_of(pool, real_path).await?;

    let fitxer_query = sqlx::query!(
        r#"
        INSERT OR IGNORE INTO fitxers (full_path, tipus_id)
                         VALUES (?, ?)
"#,
    )
    .bind(db_path.to_string_lossy())
    .bind(tipus_id)
    .execute(pool)
    .await?;

    if fitxer_query.rows_affected() > 0 {
        sqlx::query(
            r#"
        INSERT INTO hashes (hash_id, short_hash_1mb, full_hash)
                         VALUES (?, ?, ?)
"#,
        )
        .bind(fitxer_query.last_insert_rowid())
        .bind(short_hash)
        .bind(full_hash)
        .execute(pool)
        .await?;
    }

    Ok(())
}
