use recopilatori::{geoloc::update_geoloc, *};
use regex::Regex;
use std::{
    fs, io,
    path::{Path, PathBuf},
    process,
    sync::mpsc,
    thread,
    time::Instant,
};

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

/// Make database reflect state of `folder`
async fn populate(
    pool: &SqlitePool,
    folder: &str,
    ignore_patterns: Vec<Regex>,
) -> Result<(), sqlx::Error> {
    let start_time: DateTime<Utc> = Utc::now();
    let (tx, rx) = mpsc::channel();

    async fn hash_files(
        pool: &SqlitePool,
        rx: mpsc::Receiver<fs::DirEntry>,
        folder: &str,
        ignore_patterns: Vec<Regex>,
        start_time: DateTime<Utc>,
    ) -> Result<(), sqlx::Error> {
        while let Ok(file) = rx.recv() {
            let curr_time: DateTime<Utc> = Utc::now();

            let real_path = file.path();
            let db_path = file.path().to_owned();
            let db_path = db_path.strip_prefix(folder).unwrap_or_else(|_| {
                error("Error intern: fitxer de la carpeta no està dins de la carpeta?");
                process::exit(1)
            });

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

            let start_hash = Instant::now();
            let (short_hash, full_hash) = hashes_of(&file_contents);
            let end_hash = Instant::now();
            inform(&format!(
                "Hash trobada, tardant: '{:?}'",
                end_hash - start_hash
            ));

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

            let delta = Utc::now() - curr_time;
            inform(&format!(
                "Processing file {} took '{ANSIITALIC}{delta}{ANSICLEAR}'",
                db_path.display()
            ));

            eprintln!();

            inform("Marking those not seen as deleted...");
            mark_not_seen_as_deleted(pool, &start_time).await?;
            inform("Finished marking those not seen as deleted");
        }
        Ok::<(), sqlx::Error>(())
    }

    let hasher_handle =
        tokio::spawn(hash_files(pool, rx, folder, ignore_patterns, start_time)).await;

    let reader_handle = thread::spawn(move || {
        for file in recurse_files(Path::new(folder))? {
            match tx.send(file) {
                Ok(()) => {}
                Err(e) => {
                    error("Error sending to hashing thread: {e}");
                    std::process::exit(1);
                }
            };
        }
        Ok::<(), sqlx::Error>(())
    });

    reader_handle.join();
    match hasher_handle {
        Ok(h) => h?,
        Err(e) => {
            error(&format!("Error fent hash!: {e}"));
            std::process::exit(2);
        }
    }

    Ok(())
}

async fn existance_check(pool: &SqlitePool, folder: &str) -> Result<(), sqlx::Error> {
    for file in recurse_files(&Path::new(&folder))? {
        let start_time = Instant::now();
        let matches = existeix(pool, &file.path()).await?;
        if matches.len() > 0 {
            report(&format!(
                "{}\t{ANSIYELLOW}DUPLICAT{ANSICLEAR}\t[{}]",
                file.path().display(),
                matches.join(", ")
            ));
        } else {
            report(&format!(
                "{}\t{ANSIGREEN}NOU{ANSICLEAR}",
                file.path().display()
            ));
        }
        let end_time = Instant::now();
        inform(&format!(
            "Checking existance of {} took '{ANSIITALIC}{:#?}{ANSICLEAR}'",
            file.file_name().to_string_lossy(),
            end_time - start_time
        ));

        eprintln!();
    }
    Ok(())
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
                    println!("ERROR: regex invàlida al fitxer d'ignorats: '{e}'");
                    std::process::exit(2);
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
    let db_url = dotenv::var("DATABASE_URL").unwrap_or_else(|_| {
        error("Falta fitxer .env amb $DATABASE_URL (vegi README.md)");
        process::exit(2)
    });
    inform(&format!("Found DATABASE_URL: '{db_url}'\n"));

    let pool = SqlitePoolOptions::new()
        .max_connections(5)
        .connect(&db_url)
        .await
        .unwrap_or_else(|e| {
            error(&format!("No s'ha pogut obrir la BD: '{e}"));
            process::exit(1)
        });

    match cli.command {
        Some(Commands::Populate {
            path_directori_font: p,
        }) => populate(&pool, &p, ignore_patterns).await?,
        Some(Commands::Exists {
            path_fitxers_unknown: p,
        }) => existance_check(&pool, &p).await?,
        Some(Commands::Geoloc {
            path_directori_font,
        }) => update_geoloc(&pool, &PathBuf::from(&path_directori_font)).await?,
        Some(Commands::ClearAllYesImVerySureNukeItAll) => clear_all(&pool).await?,
        None => {
            println!("T'has deixat la subcomanda (--help per veure-les)");
            std::process::exit(1);
        }
    }
    Ok(())
}
