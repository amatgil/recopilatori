CREATE TABLE IF NOT EXISTS tipus_fitxers (
    tipus_id INTEGER PRIMARY KEY,
    tipus_nom TEXT NOT NULL UNIQUE
);
CREATE TABLE IF NOT EXISTS fitxers (
    fitxer_id INTEGER PRIMARY KEY,
    full_path TEXT NOT NULL UNIQUE,
    tipus_id INTEGER REFERENCES tipus_fitxers,
    fitxer_size INTEGER NOT NULL,
    last_scanned TEXT NOT NULL,
    is_deleted BOOLEAN
);

CREATE TABLE IF NOT EXISTS hashes (
    fitxer_id PRIMARY KEY REFERENCES fitxers,
    short_hash_1mb UUID NOT NULL,
    full_hash UUID NOT NULL
);

CREATE TABLE IF NOT EXISTS coords (
    fitxer_id PRIMARY KEY REFERENCES fitxers,
    latitude REAL NOT NULL,
    longitude REAL NOT NULL
);

