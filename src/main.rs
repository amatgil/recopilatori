use recopilatori::{
    clear_all, existance::existance_check, geoloc::update_geoloc, get_ignore_patterns, inform,
    oopsie, populating::populate,
};
use std::path::PathBuf;
use std::sync::Arc;

use sqlx::sqlite::SqlitePoolOptions;

use clap::{Parser, Subcommand};

#[derive(Subcommand, Debug)]
enum Commands {
    /// Actualitza la base de dades (`./dades.db`) amb tots els fitxers continguts al `path_directori_font`
    Populate { path_directori_font: String },
    /// Comprova, per cada fitxer de `path_directori_unknown`, si existeix ja a la base de dades (`./dades.db`)
    Exists { path_fitxers_unknown: String },
    /// Actualitza les coordenades dels fitxers que en contenen
    Geoloc { path_directori_font: String },
    /// Delete all data from the datable (DELETE FROM all tables)
    ClearAllYesImVerySureNukeItAll,
}

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
#[command(propagate_version = true)]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
}

#[tokio::main]
async fn main() -> Result<(), sqlx::Error> {
    let cli = Cli::parse();
    let ignore_patterns = get_ignore_patterns()?;
    let db_url = dotenv::var("DATABASE_URL")
        .unwrap_or_else(|_| oopsie("Falta fitxer .env amb $DATABASE_URL (vegi README.md)", 2));
    inform(&format!("Found DATABASE_URL: '{db_url}'\n"));

    let pool = SqlitePoolOptions::new()
        .max_connections(5)
        .connect(&db_url)
        .await
        .unwrap_or_else(|e| oopsie(&format!("No s'ha pogut obrir la BD: '{e}"), 1));

    match cli.command {
        Some(Commands::Populate {
            path_directori_font: p,
        }) => populate(pool, Arc::new(p), Arc::new(ignore_patterns)).await?,
        Some(Commands::Exists {
            path_fitxers_unknown: p,
        }) => existance_check(pool, p).await?,
        Some(Commands::Geoloc {
            path_directori_font,
        }) => update_geoloc(&pool, &PathBuf::from(&path_directori_font)).await?,
        Some(Commands::ClearAllYesImVerySureNukeItAll) => clear_all(&pool).await?,
        None => oopsie("T'has deixat la subcomanda (--help per veure-les)", 1),
    }
    Ok(())
}
