use recopilatori::*;
use std::{path::Path, time::Instant};

use sqlx::sqlite::*;

use clap::*;

#[derive(Subcommand, Debug)]
enum Commands {
    /// Actualitza la base de dades (`./dades.db`) amb tots els fitxers continguts al `path_directori_font`
    Populate { path_directori_font: String },
    /// Comprova, per cada fitxer de `path_directori_unknown`, si existeix ja a la base de dades (`./dades.db`)
    Exists { path_fitxers_unknown: String },
}

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
#[command(propagate_version = true)]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
}

async fn populate(folder: &str) -> Result<(), sqlx::Error> {
    // Create a connection pool
    let pool = SqlitePoolOptions::new()
        .max_connections(5)
        .connect("sqlite://dades.db")
        .await?;

    setup(&pool).await?;

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

#[tokio::main]
async fn main() -> Result<(), sqlx::Error> {
    let cli = Cli::parse();
    match cli.command {
        Some(Commands::Populate {
            path_directori_font: p,
        }) => populate(&p).await?,
        Some(Commands::Exists {
            path_fitxers_unknown: p,
        }) => {
            for file in recurse_files(&Path::new(&p))? {
                if let Some(preexisting) = existeix(&file.path()).await? {
                    println!(
                        "{}:\tDUPLICAT\t{}",
                        file.path().display(),
                        preexisting.display()
                    );
                } else {
                    println!("{}:\tNOU", file.path().display());
                }
            }
        }
        None => {
            println!("T'has deixat la subcomanda (--help per veure-les)");
            std::process::exit(1);
        }
    }
    Ok(())
}
