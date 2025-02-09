# Recopilador

Assumes (possible empty) file called `dades.db` exists at callsite

## Usage
- `recopilador populate <folder>`: populate DB with files and hashes of recursive contents of `<folder>`
- `recopilador exists <folder>`: check if files in `<folder>` exist in DB (first by short hash, then full hash, then by full contents), returning whether they exist
