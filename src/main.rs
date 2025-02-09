use sqlx::sqlite::*;

async fn setup(pool: SqlitePool) -> Result<(), sqlx::Error> {
    let create = sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS tipus_fitxers (
            tipus_id INTEGER PRIMARY KEY,
            tipus_nom TEXT NOT NULL
        );

        CREATE TABLE IF NOT EXISTS fitxers (
            fitxer_id INTEGER PRIMARY KEY,
            full_path TEXT NOT NULL,
            tipus_id INTEGER NOT NULL REFERENCES tipus_fitxers
        );

        CREATE TABLE IF NOT EXISTS hashes (
            hash_id INTEGER PRIMARY KEY,
            short_hash_1mb UUID NOT NULL,
            full_hash UUID NOT NULL
        );
        "#,
    )
    .execute(&pool)
    .await?;
    Ok(())
}

#[tokio::main]
async fn main() -> Result<(), sqlx::Error> {
    // Create a connection pool
    let pool = SqlitePoolOptions::new()
        .max_connections(5)
        .connect("sqlite://dades.db")
        .await?;
    setup(pool).await?;

    /*
    let account = sqlx::query("select *").fetch_all(&pool).await?;
    dbg!(account);*/

    //println!("{account:?}");
    Ok(())
}
