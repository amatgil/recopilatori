use crate::{error, fs, inform, log, short_hash_of, Path};
use sqlx::{
    types::chrono::{DateTime, Utc},
    types::uuid::Uuid,
    FromRow, Result, SqlitePool,
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
    match path.extension() {
        None => Ok(None),
        Some(ext) => {
            let ext = ext.to_string_lossy().to_ascii_lowercase();
            let s = sqlx::query!("SELECT * FROM tipus_fitxers WHERE tipus_nom = ?", ext)
                .fetch_optional(pool)
                .await?;

            match s {
                Some(ret) => Ok(ret.tipus_id),
                None => {
                    let q = sqlx::query!("INSERT INTO tipus_fitxers (tipus_nom) VALUES (?)", ext)
                        .execute(pool)
                        .await?;
                    if q.rows_affected() == 0 {
                        error("Extension both existed and then didn't exist")
                    }
                    Ok(Some(q.last_insert_rowid()))
                }
            }
        }
    }
}

pub async fn insert_file(
    pool: &SqlitePool,
    real_path: &Path,
    db_path: &Path,
    short_hash: [u8; 16],
    full_hash: [u8; 16],
    file_size: i64,
    scan_time: DateTime<Utc>,
) -> Result<(), sqlx::Error> {
    inform("Insertant a BD...");
    let tipus_id = get_tipus_id_of(pool, real_path).await?;
    log(&format!(
        "\tValor és: ({}, {:?}, {:x?})",
        db_path.display(),
        tipus_id,
        short_hash
    ));

    let db_path = db_path.to_string_lossy();
    let short_hash = sqlx::types::Uuid::from_slice(&short_hash).expect("invalid hash provided");
    let full_hash = sqlx::types::Uuid::from_slice(&full_hash).expect("invalid hash provided");

    log(&format!(
        "Query es: INSERT OR REPLACE INTO fitxers (full_path, tipus_id, last_scanned, fitxer_size, is_deleted) VALUES ({}, {:?}, {}, {}, FALSE);" ,
        db_path, tipus_id, scan_time, file_size,
    ));

    let fitxer_query = sqlx::query!(
        r#"
        INSERT OR REPLACE INTO fitxers (full_path, tipus_id, last_scanned, fitxer_size, is_deleted)
                         VALUES (?, ?, ?, ?, FALSE);
        "#,
        db_path,
        tipus_id,
        scan_time,
        file_size,
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
    pool: SqlitePool,
    original_time: &DateTime<Utc>,
) -> Result<(), sqlx::Error> {
    inform("Marking those not seen as deleted...");
    sqlx::query!(
        r#"
        UPDATE fitxers SET is_deleted = true WHERE last_scanned < ?;
        "#,
        original_time
    )
    .execute(&pool)
    .await?;
    inform("Finished marking those not seen as deleted");

    Ok(())
}

/// Returns Strings that are paths (if the real one isn't uf8, it was lossily converted)
pub async fn existeix(pool: &SqlitePool, new_p: &Path) -> Result<Vec<String>, sqlx::Error> {
    inform(&format!("Comprovant si '{}' existeix", new_p.display()));

    let new_contents = fs::read(new_p)?;
    let new_file_size: i64 = new_contents.len() as i64;

    let possible_matches = sqlx::query!(
        r#"
        SELECT f.full_path, h.short_hash_1mb "short_hash_1mb: Uuid"
        FROM fitxers f, hashes h
        WHERE f.fitxer_id = h.fitxer_id AND f.fitxer_size = ?;
"#,
        new_file_size
    )
    .fetch_all(pool)
    .await?;

    inform(&format!(
        "{} possibles candidades found from size",
        possible_matches.len()
    ));

    if possible_matches.is_empty() {
        inform("no equal files");
        Ok(vec![])
    } else {
        let current_short_hash = sqlx::types::Uuid::from_slice(&short_hash_of(&new_contents))
            .expect("short_hash_of did not return valid uuid??");

        let mut r = vec![];
        for m in possible_matches {
            let preexisting_path = m.full_path;
            let preexisting_shorthash = m.short_hash_1mb;
            if current_short_hash == preexisting_shorthash {
                r.push(preexisting_path);
            }
        }

        inform(&format!(
            "{} of those possibles candidades matches short hash and file size",
            r.len()
        ));

        Ok(r)
    }
}
