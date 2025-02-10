use recopilatori::*;
use std::{path::Path, time::Instant};

use sqlx::sqlite::*;

use clap::*;

#[derive(Subcommand, Debug)]
enum Commands {
    /// Actualitza la base de dades (`./dades.db`) amb tots els fitxers continguts al `path_directori_font`
    Populate {
        path_directori_font: String,
    },
    /// Comprova, per cada fitxer de `path_directori_unknown`, si existeix ja a la base de dades (`./dades.db`)
    Exists {
        path_fitxers_unknown: String,
    },
    ClearAllYesImVerySureNukeItAll,
}

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
#[command(propagate_version = true)]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
}

async fn populate(pool: &SqlitePool, folder: &str) -> Result<(), sqlx::Error> {
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
        insert_file(&pool, &real_path, db_path, short_hash, full_hash).await?;
    }
    Ok(())
}

async fn existance_check(pool: &SqlitePool, folder: &str) -> Result<(), sqlx::Error> {
    for file in recurse_files(&Path::new(&folder))? {
        let matches = existeix(pool, &file.path()).await?;
        if matches.len() > 0 {
            report(&format!(
                "{}:\tDUPLICAT\t[{}]",
                file.path().display(),
                matches.join(", ")
            ));
        } else {
            report(&format!("{}:\tNOU", file.path().display()));
        }
    }
    Ok(())
}

#[tokio::main]
async fn main() -> Result<(), sqlx::Error> {
    let cli = Cli::parse();
    let pool = SqlitePoolOptions::new()
        .max_connections(5)
        .connect("sqlite://dades.db")
        .await?;

    match cli.command {
        Some(Commands::Populate {
            path_directori_font: p,
        }) => populate(&pool, &p).await?,
        Some(Commands::Exists {
            path_fitxers_unknown: p,
        }) => existance_check(&pool, &p).await?,
        Some(Commands::ClearAllYesImVerySureNukeItAll) => clear_all(&pool).await?,
        None => {
            println!("T'has deixat la subcomanda (--help per veure-les)");
            std::process::exit(1);
        }
    }
    Ok(())
}
