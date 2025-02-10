use recopilatori::*;
use regex::Regex;
use std::{fs, io, path::Path, time::Instant};

use sqlx::{
    sqlite::*,
    types::chrono::{DateTime, Utc},
};

use clap::*;

#[derive(Subcommand, Debug)]
enum Commands {
    /// Actualitza la base de dades (`./dades.db`) amb tots els fitxers continguts al `path_directori_font`
    Populate { path_directori_font: String },
    /// Comprova, per cada fitxer de `path_directori_unknown`, si existeix ja a la base de dades (`./dades.db`)
    Exists { path_fitxers_unknown: String },
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

/// Make database reflect state of `folder`
async fn populate(
    pool: &SqlitePool,
    folder: &str,
    ignore_patterns: Vec<Regex>,
) -> Result<(), sqlx::Error> {
    let start_time: DateTime<Utc> = Utc::now();

    for file in recurse_files(Path::new(folder))? {
        let curr_time: DateTime<Utc> = Utc::now();

        let real_path = file.path();
        let db_path = file.path().to_owned();
        let db_path = db_path
            .strip_prefix(folder)
            .expect("Error intern: fitxer de la carpeta no està dins de la carpeta?");

        if let Some(r) = ignore_patterns
            .iter()
            .filter(|r| r.is_match(&db_path.display().to_string()))
            .next()
        {
            inform(&format!(
                "Ignoring file '{}' (per regex '{}')",
                db_path.display(),
                r
            ));
            continue;
        }
        let file_contents: Vec<u8> = fs::read(&real_path)?;

        inform(&format!("Tractant: {:?}", db_path));

        let start = Instant::now();
        let (short_hash, full_hash) = hashes_of(&file_contents);
        let end = Instant::now();
        inform(&format!("Hash trobada, tardant: '{:?}'", end - start));

        inform("Insertant a BD...");
        insert_file(
            &pool,
            &real_path,
            db_path,
            short_hash,
            full_hash,
            file_contents.len() as i64,
            curr_time,
        )
        .await?;
    }

    mark_not_seen_as_deleted(pool, start_time).await?;

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
    let ignore_patterns = match match fs::read_to_string("recopilatori.ignored") {
        Ok(c) => c
            .split('\n')
            .filter(|s| !s.is_empty())
            .map(|s| Regex::new(s))
            .collect::<Result<Vec<Regex>, _>>(),
        Err(e) if e.kind() == io::ErrorKind::NotFound => {
            inform("No `recopilatori.ignored` detected");
            Ok(vec![])
        }
        e => {
            e?;
            unreachable!()
        }
    } {
        Ok(p) => p,
        Err(e) => {
            println!("ERROR: regex invàlida al fitxer d'ignorats {e}");
            std::process::exit(2);
        }
    };

    let pool = SqlitePoolOptions::new()
        .max_connections(5)
        .connect("sqlite://dades.db")
        .await?;

    match cli.command {
        Some(Commands::Populate {
            path_directori_font: p,
        }) => populate(&pool, &p, ignore_patterns).await?,
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
