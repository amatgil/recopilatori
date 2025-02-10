# Recopilador

Assumes (possible empty) sqlite database called `dades.db` exists at callsite

## Usage
- `recopilador populate <folder>`: populate DB with files and hashes of recursive contents of `<folder>`
- `recopilador exists <folder>`: check if files in `<folder>` exist in DB (first by short hash, then full hash, then by full contents), returning whether they exist

## Schema intern
```
CREATE TABLE IF NOT EXISTS tipus_fitxers (
    tipus_id INTEGER PRIMARY KEY,
    tipus_nom TEXT NOT NULL UNIQUE
);
CREATE TABLE IF NOT EXISTS fitxers (
    fitxer_id INTEGER PRIMARY KEY,
    full_path TEXT NOT NULL UNIQUE,
    tipus_id INTEGER REFERENCES tipus_fitxers
);

CREATE TABLE IF NOT EXISTS hashes (
    hash_id PRIMARY KEY,
    short_hash_1mb UUID NOT NULL,
    full_hash UUID NOT NULL
);
```
