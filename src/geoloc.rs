use sqlx::SqlitePool;

use crate::*;
use nom_exif::{ExifIter, LatLng, MediaParser, MediaSource, URational};

/// Returns (lat, long)
fn get_latlong(f: &Path) -> Result<Option<(f64, f64)>, nom_exif::Error> {
    let mut parser = MediaParser::new();
    let data = fs::File::open(f)?;
    let ms = MediaSource::file(data)?;

    if ms.has_exif() {
        let exif: ExifIter = parser.parse(ms)?;
        match exif.parse_gps_info() {
            Ok(Some(info)) => {
                let r_to_f = |r: URational| r.0 as f64 / r.1 as f64;
                let dms_to_f = |l: LatLng| r_to_f(l.0) + r_to_f(l.1) / 60.0 + r_to_f(l.2) / 3600.0;
                let lat = dms_to_f(info.latitude);
                let lon = dms_to_f(info.longitude);
                Ok(Some((lat, lon)))
            }
            Ok(None) => Ok(None),
            Err(e) => Err(e),
        }
    } else {
        Ok(None)
    }
}

pub async fn update_geoloc(pool: &SqlitePool, dir: &Path) -> Result<(), sqlx::Error> {
    let inner_paths =
        sqlx::query!("SELECT fitxer_id, full_path FROM fitxers WHERE is_deleted = FALSE;")
            .fetch_all(pool)
            .await?;

    for inner_rec in inner_paths {
        let inner_path = inner_rec.full_path;
        let real_path = dir.join(&Path::new(&inner_path));
        inform(&format!(
            "Looking for metadata of {inner_path} ({})",
            real_path.display()
        ));
        match get_latlong(&real_path) {
            Ok(Some((lat, long))) => {
                inform(&format!("Found data: ({lat}, {long})"));
                sqlx::query!(
                    "INSERT OR REPLACE INTO coords (fitxer_id, latitude, longitude) VALUES (?, ?, ?);",
                    inner_rec.fitxer_id,
                    lat,
                    long
                )
                .execute(pool)
                .await?;
            }
            _ => inform("No location data found"),
        }
    }

    Ok(())
}
