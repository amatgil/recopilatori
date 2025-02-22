# Recopilador

Assumes (possible empty) sqlite database called `dades.db` exists at callsite

## Usage
- `recopilador populate <folder>`: populate DB with files and hashes of recursive contents of `<folder>`
- `recopilador exists <folder>`: check if files in `<folder>` exist in DB (first by short hash, then full hash, then by full contents), returning whether they exist

Necessita de dos fitxers amb les següents dades:
- `.env`: informació, configuració
```sh
DATABASE_URL=sqlite://dades.db # per exemple, base de dades serà `./dades.db`
```
- `recopilador.ignore`: llista de regex a ignorar, una per linia. La regex s'executa per tot el path, no només el nom del fitxer
```sh
*.sensitive_extension
```

## Init 
```
sqlite3 <desired-database-name> < schema.sql
```
