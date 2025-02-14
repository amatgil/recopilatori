use recopilatori::{geoloc::update_geoloc, *};
use regex::Regex;
use std::{fs, io, path::PathBuf};

use existance::*;
use populating::*;

use sqlx::sqlite::*;

use clap::*;

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
    let ignore_patterns: Vec<Regex> = match fs::read_to_string("recopilatori.ignored") {
        Ok(c) => {
            let r = c
                .split('\n')
                .filter(|s| !s.is_empty())
                .map(|s| Regex::new(s))
                .collect::<Result<Vec<Regex>, _>>()
                .unwrap_or_else(|e| {
                    oopsie(
                        &format!("ERROR: regex invàlida al fitxer d'ignorats: '{e}'",),
                        1,
                    )
                });

            inform(&format!(
                "recopilatori.ignored detectat amb '{}' patrons\n",
                r.len()
            ));
            r
        }
        Err(e) if e.kind() == io::ErrorKind::NotFound => {
            inform("No `recopilatori.ignored` detected\n");
            vec![]
        }
        e => {
            e?;
            unreachable!()
        }
    };
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
        }) => populate(pool, p, ignore_patterns).await?,
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
