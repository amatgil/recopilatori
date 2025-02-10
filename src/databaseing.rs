use crate::*;
use sqlx::*;

#[derive(FromRow, Debug, Clone)]
pub struct TipusFitxer {
    pub tipus_id: u32,
    pub tipus_nom: String,
}

pub async fn setup(pool: &SqlitePool) -> Result<(), sqlx::Error> {
    inform("Creating tables if they don't exist...");
    sqlx::query!(
        r#"
        CREATE TABLE IF NOT EXISTS tipus_fitxers (
            tipus_id INTEGER PRIMARY KEY,
            tipus_nom TEXT NOT NULL UNIQUE
        );"#,
    )
    .execute(pool)
    .await?;

    sqlx::query!(
        r#"
        CREATE TABLE IF NOT EXISTS fitxers (
            fitxer_id INTEGER PRIMARY KEY,
            full_path TEXT NOT NULL UNIQUE,
            tipus_id INTEGER REFERENCES tipus_fitxers
        );"#,
    )
    .execute(pool)
    .await?;

    sqlx::query!(
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
        let ext = ext.to_string_lossy();
        dbg!(&ext);
        let q = sqlx::query!(
            "INSERT OR IGNORE INTO tipus_fitxers (tipus_nom) VALUES (?)",
            ext
        )
        .execute(pool)
        .await?;

        if q.rows_affected() > 0 {
            Ok(Some(q.last_insert_rowid()))
        } else {
            let r = sqlx::query!(
                "SELECT tipus_id FROM tipus_fitxers t WHERE t.tipus_nom = ?",
                ext
            )
            .fetch_one(pool)
            .await?;
            Ok(r.tipus_id)
        }
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
    dbg!(&real_path, &tipus_id);

    let db_path = db_path.to_string_lossy();
    let short_hash = sqlx::types::Uuid::from_slice(&short_hash).expect("invalid hash provided");
    let full_hash = sqlx::types::Uuid::from_slice(&full_hash).expect("invalid hash provided");

    let fitxer_query = sqlx::query!(
        r#"
        INSERT OR IGNORE INTO fitxers (full_path, tipus_id)
                         VALUES (?, ?)
"#,
        db_path,
        tipus_id
    )
    .execute(pool)
    .await?;

    if fitxer_query.rows_affected() > 0 {
        let hash_id = fitxer_query.last_insert_rowid();
        sqlx::query!(
            r#"
        INSERT INTO hashes (hash_id, short_hash_1mb, full_hash)
                         VALUES (?, ?, ?)
"#,
            hash_id,
            short_hash,
            full_hash
        )
        .execute(pool)
        .await?;
    }

    Ok(())
}

fn files_are_equal(a: &Path, b: &Path) -> bool {
    todo!()
}

/// Returns Strings that are paths (if the real one isn't uf8, it was lossily converted)
pub async fn existeix(pool: &SqlitePool, p: &Path) -> Result<Vec<String>, sqlx::Error> {
    let current_hash = sqlx::types::Uuid::from_slice(&short_hash_of(&fs::read(p)?))
        .expect("short_hash did not return valid uuid??");
    let matches = sqlx::query!(
        r#"
        SELECT f.full_path
        FROM fitxers f, hashes h
        WHERE f.tipus_id = h.hash_id AND h.short_hash_1mb = ?
"#,
        current_hash
    )
    .fetch_all(pool)
    .await?;

    Ok(matches.into_iter().map(|r| r.full_path).collect())
}
