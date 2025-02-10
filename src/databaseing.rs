use crate::*;
use sqlx::{
    types::chrono::{DateTime, Utc},
    *,
};

#[derive(FromRow, Debug, Clone)]
pub struct TipusFitxer {
    pub tipus_id: u32,
    pub tipus_nom: String,
}

pub async fn clear_all(pool: &SqlitePool) -> Result<(), sqlx::Error> {
    inform("Deleting all data...");
    sqlx::query!("DELETE FROM hashes;").execute(pool).await?;
    sqlx::query!("DELETE FROM fitxers;").execute(pool).await?;
    sqlx::query!("DELETE FROM tipus_fitxers;")
        .execute(pool)
        .await?;
    Ok(())
}

pub async fn get_tipus_id_of(pool: &SqlitePool, path: &Path) -> Result<Option<i64>, sqlx::Error> {
    if let Some(ext) = path.extension() {
        let ext = ext.to_string_lossy();
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
    scan_time: DateTime<Utc>,
) -> Result<(), sqlx::Error> {
    let tipus_id = get_tipus_id_of(pool, real_path).await?;

    let db_path = db_path.to_string_lossy();
    let short_hash = sqlx::types::Uuid::from_slice(&short_hash).expect("invalid hash provided");
    let full_hash = sqlx::types::Uuid::from_slice(&full_hash).expect("invalid hash provided");

    let fitxer_query = sqlx::query!(
        r#"
        INSERT OR IGNORE INTO fitxers (full_path, tipus_id, last_scanned, is_deleted)
                         VALUES (?, ?, ?, FALSE);
        "#,
        db_path,
        tipus_id,
        scan_time
    )
    .execute(pool)
    .await?;

    sqlx::query!(
        r#"
        UPDATE fitxers SET last_scanned = ? WHERE full_path = ?;
        "#,
        scan_time,
        db_path
    )
    .execute(pool)
    .await?;

    if fitxer_query.rows_affected() > 0 {
        let hash_id = fitxer_query.last_insert_rowid();
        sqlx::query!(
            r#"
        INSERT INTO hashes (fitxer_id, short_hash_1mb, full_hash)
                         VALUES (?, ?, ?);
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

pub async fn mark_not_seen_as_deleted(
    pool: &SqlitePool,
    original_time: DateTime<Utc>,
) -> Result<(), sqlx::Error> {
    sqlx::query!(
        r#"
        UPDATE fitxers SET is_deleted = true WHERE last_scanned < ?;
        "#,
        original_time
    )
    .execute(pool)
    .await?;

    Ok(())
}

/// Returns Strings that are paths (if the real one isn't uf8, it was lossily converted)
pub async fn existeix(pool: &SqlitePool, new_p: &Path) -> Result<Vec<String>, sqlx::Error> {
    inform(&format!("Comprovant si '{}' existeix", new_p.display()));

    let new_contents = fs::read(new_p)?;
    let current_short_hash = sqlx::types::Uuid::from_slice(&short_hash_of(&new_contents))
        .expect("short_hash_of did not return valid uuid??");

    let possible_matches = sqlx::query!(
        r#"
        SELECT f.fitxer_id, f.full_path
        FROM fitxers f, hashes h
        WHERE f.fitxer_id = h.fitxer_id AND h.short_hash_1mb = ?
"#,
        current_short_hash
    )
    .fetch_all(pool)
    .await?;

    inform(&format!(
        "{} possibles candidades found from short hash",
        possible_matches.len()
    ));

    let mut r = vec![];
    for m in possible_matches {
        let preexisting_path = m.full_path;
        let preexisting_content = fs::read(&preexisting_path)?;
        if preexisting_content == new_contents {
            r.push(preexisting_path);
        }
    }

    inform(&format!(
        "{} of those possibles candidades matches file conents",
        r.len()
    ));

    Ok(r)
}
