// use parking_lot::{MappedMutexGuard, Mutex, MutexGuard};
use std::collections::HashMap;
use std::ffi::OsStr;
use std::path::{Path, PathBuf};
// use std::sync::Arc;
use hotwatch::{Event, EventKind, Hotwatch};
use std::sync::Mutex;

use r2d2_sqlite::SqliteConnectionManager;
use rusqlite::{params, OpenFlags};

use serde::{Deserialize, Serialize};
use serde_json::Value as JSONValue;

use crate::errors::{Error, Result};
use crate::pmtiles::{self, Compression, Pmtiles, TileType};
use std::io::Read;
use std::sync::Arc;

use crate::utils::{decode, get_data_format, get_tile_format, DataFormat};

pub type Connection = r2d2::PooledConnection<SqliteConnectionManager>;

/// Where the tiles of an open file actually live.
#[derive(Clone, Debug)]
pub enum Backend {
  Mbtiles(r2d2::Pool<SqliteConnectionManager>),
  Pmtiles(Arc<Pmtiles>),
}

#[derive(Clone, Debug)]
pub struct TileMeta {
  pub backend: Backend,
  pub path: PathBuf,
  pub name: Option<String>,
  pub version: Option<String>,
  pub tilejson: String,
  pub scheme: String,
  pub id: String,
  pub tile_format: DataFormat,
  pub grid_format: Option<DataFormat>,
  pub bounds: Option<Vec<f64>>,
  pub center: Option<Vec<f64>>,
  pub minzoom: Option<u32>,
  pub maxzoom: Option<u32>,
  pub description: Option<String>,
  pub attribution: Option<String>,
  pub layer_type: Option<String>,
  pub legend: Option<String>,
  pub template: Option<String>,
  pub json: Option<JSONValue>,
}

impl TileMeta {
  /// Tile bytes for an XYZ address, as stored (a vector tile may still be gzipped).
  /// `Ok(None)` is a tile the file simply does not have.
  pub fn get_tile(&self, z: u32, x: u32, y: u32) -> Result<Option<Vec<u8>>> {
    match &self.backend {
      Backend::Mbtiles(pool) => {
        // MBTiles stores rows bottom-up (TMS)
        let tms_y = match 1u32.checked_shl(z).and_then(|n| n.checked_sub(1 + y)) {
          Some(row) => row,
          None => return Ok(None),
        };
        let connection = pool.get().map_err(Error::Pool)?;
        get_tile_data(&connection, z, x, tms_y)
      }
      Backend::Pmtiles(archive) => {
        let raw = match u8::try_from(z) {
          Ok(z) => archive.get_tile(z, x, y)?,
          Err(_) => return Ok(None),
        };
        match raw {
          // the service layer sniffs and inflates vector tiles itself; images are never compressed
          Some(data) => Ok(Some(pmtiles::decompress(
            data,
            match archive.header.tile_compression {
              Compression::Gzip if self.tile_format.is_vector() => Compression::None,
              other => other,
            },
          )?)),
          None => Ok(None),
        }
      }
    }
  }
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TileSummaryJSON {
  pub image_type: DataFormat,
  pub url: String,
}

// #[derive(Deserialize)]
// struct UTFGridKeys {
//   pub grid: Vec<String>,
//   pub keys: Vec<String>,
// }

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UTFGrid {
  pub data: HashMap<String, JSONValue>,
  pub grid: Vec<String>,
  pub keys: Vec<String>,
}

pub trait CloneableFn: Fn(String) -> () + Send {
  fn clone_box<'a>(&self) -> Box<dyn 'a + CloneableFn>
  where
    Self: 'a;
}

impl<F: Fn(String) -> () + Clone + Send> CloneableFn for F {
  fn clone_box<'a>(&self) -> Box<dyn 'a + CloneableFn>
  where
    Self: 'a,
  {
    Box::new(self.clone())
  }
}

impl<'a> Clone for Box<dyn 'a + CloneableFn> {
  fn clone(&self) -> Self {
    (**self).clone_box()
  }
}

#[derive(Clone)]
struct TilesetsData {
  pub data: Option<TileMeta>,
  pub path: PathBuf,
  pub callback: Box<dyn CloneableFn>,
}

// #[derive()]
// pub struct Tilesets {
//   watcher: Hotwatch,
//   data: Arc<Mutex<HashMap<String, TilesetsData>>>,
// }

lazy_static! {
  static ref WATCHER: Mutex<Hotwatch> =
    Mutex::new(Hotwatch::new().expect("hotwatch failed to initialize!"));
  static ref TILESET_MAP: Mutex<HashMap<String, TilesetsData>> = Mutex::new(HashMap::new());
}

pub fn get_data(key: &String) -> Option<TileMeta> {
  TILESET_MAP
    .lock()
    .unwrap_or_else(|poisoned| poisoned.into_inner())
    .get(key)
    .and_then(|entry| entry.data.clone())
}

/// A panic elsewhere must not turn every later request into one: the map is plain data, so a
/// poisoned lock is still safe to read and write.
fn lock_map() -> std::sync::MutexGuard<'static, HashMap<String, TilesetsData>> {
  TILESET_MAP
    .lock()
    .unwrap_or_else(|poisoned| poisoned.into_inner())
}

fn lock_watcher() -> std::sync::MutexGuard<'static, Hotwatch> {
  WATCHER
    .lock()
    .unwrap_or_else(|poisoned| poisoned.into_inner())
}

pub fn get_path(key: &String) -> Option<PathBuf> {
  lock_map().get(key).map(|entry| entry.path.clone())
}

fn watch_file(key: &String, path: &Path) {
  let watch_key = key.clone();
  // The watcher takes its own lock around every callback, and the callback needs the tileset map:
  // never call this while holding the map or the two threads can wait on each other forever.
  if let Err(err) = lock_watcher().watch(path, move |event: Event| {
    if let EventKind::Modify(_) = event.kind {
      reload(&watch_key);
    }
  }) {
    println!("error watching {}: {}", path.display(), err);
  }
}

/// Opens `path` and registers it under `key`. Nothing is registered when it cannot be read, so the
/// caller can report the reason instead of the map later serving a file that never loaded.
pub fn set_mbtiles(key: &String, path: PathBuf, callback: Box<dyn CloneableFn>) -> Result<()> {
  // the slow part (opening, sniffing) runs without any lock held
  let tile_meta = get_tile_details(&path)?;

  let previous = {
    let mut map = lock_map();
    let previous = map.get(key).map(|entry| entry.path.clone());
    map.insert(
      key.clone(),
      TilesetsData {
        data: Some(tile_meta),
        path: path.clone(),
        callback,
      },
    );
    previous
  };
  if let Some(previous) = previous {
    if let Err(err) = lock_watcher().unwatch(&previous) {
      println!("error unwatch {}", err);
    }
  }
  watch_file(key, &path);
  Ok(())
}

pub fn reload(key: &String) {
  let path = match get_path(key) {
    Some(path) => path,
    None => return,
  };
  // A file caught half-written fails to open, and the last change event of a long write can
  // arrive before the write is done: give it a few chances instead of serving nothing for good.
  let mut data = None;
  for attempt in 0..6 {
    match get_tile_details(&path) {
      Ok(details) => {
        data = Some(details);
        break;
      }
      Err(_) if attempt < 5 => std::thread::sleep(std::time::Duration::from_millis(400)),
      Err(_) => {}
    }
  }
  let callback = {
    let mut map = lock_map();
    match map.get_mut(key) {
      Some(entry) => {
        entry.data = data;
        entry.callback.clone()
      }
      None => return,
    }
  };
  // outside the lock: the callback talks to the webview and may come back for tile data
  callback(key.clone());
}

pub fn get_data_format_via_query(
  tile_name: &str,
  connection: &Connection,
  category: &str,
) -> Result<DataFormat> {
  let query = match category {
    "tile" => r#"SELECT tile_data FROM tiles LIMIT 1"#,
    "grid" => r#"SELECT grid_utfgrid FROM grid_utfgrid LIMIT 1"#,
    _ => {
      return Err(Error::InvalidDataFormatQueryCategory(String::from(
        tile_name,
      )))
    }
  };
  let mut statement = match connection.prepare(query) {
    Ok(s) => s,
    Err(err) => return Err(Error::DBConnection(err)),
  };
  let data_format: DataFormat = statement
    .query_row([], |row| {
      Ok(get_tile_format(&row.get::<_, Vec<u8>>(0)?))
    })
    .unwrap_or(DataFormat::Unknown);
  Ok(data_format)
}

const SQLITE_MAGIC: &[u8; 15] = b"SQLite format 3";

/// What kind of tile archive a file is, judged by its first bytes rather than its extension.
fn sniff_file(path: &Path) -> Result<Vec<u8>> {
  let mut head = vec![0u8; 16];
  let read = std::fs::File::open(path)?.read(&mut head)?;
  head.truncate(read);
  Ok(head)
}

pub fn get_tile_details(path: &Path) -> Result<TileMeta> {
  let tile_name = path
    .file_name()
    .and_then(OsStr::to_str)
    .unwrap_or("tiles")
    .to_string();
  let head = sniff_file(path)?;
  if head.starts_with(pmtiles::MAGIC) {
    get_pmtiles_details(path, &tile_name)
  } else if head.starts_with(SQLITE_MAGIC) {
    get_mbtiles_details(path, &tile_name)
  } else {
    Err(Error::UnsupportedFile(tile_name))
  }
}

fn blank_meta(backend: Backend, path: &Path, tile_name: &str, tile_format: DataFormat) -> TileMeta {
  TileMeta {
    backend,
    path: PathBuf::from(path),
    name: None,
    version: None,
    tilejson: String::from("2.1.0"),
    scheme: String::from("xyz"),
    id: String::from(tile_name),
    tile_format,
    grid_format: None,
    bounds: None,
    center: None,
    minzoom: Some(0),
    maxzoom: Some(19),
    description: None,
    attribution: None,
    layer_type: None,
    legend: None,
    template: None,
    json: None,
  }
}

fn json_string(value: &JSONValue, key: &str) -> Option<String> {
  match value.get(key) {
    Some(JSONValue::String(text)) => Some(text.clone()),
    Some(JSONValue::Number(number)) => Some(number.to_string()),
    _ => None,
  }
}

fn get_pmtiles_details(path: &Path, tile_name: &str) -> Result<TileMeta> {
  let archive = Arc::new(Pmtiles::open(path)?);
  let header = archive.header.clone();

  let mut tile_format = match header.tile_type {
    TileType::Mvt => DataFormat::Pbf,
    TileType::Mlt => DataFormat::Mlt,
    TileType::Png => DataFormat::Png,
    TileType::Jpeg => DataFormat::Jpg,
    TileType::Webp => DataFormat::Webp,
    TileType::Avif => DataFormat::Avif,
    TileType::Unknown => DataFormat::Unknown,
  };
  if tile_format == DataFormat::Unknown {
    // the header does not say: look at the first tile that can be found
    tile_format = (0..=header.max_zoom)
      .find_map(|z| archive.get_tile(z, 0, 0).ok().flatten())
      .and_then(|raw| {
        let stored = pmtiles::decompress(raw, header.tile_compression).ok()?;
        Some(get_tile_format(&stored))
      })
      .unwrap_or(DataFormat::Pbf);
  }

  let mut metadata = blank_meta(Backend::Pmtiles(archive.clone()), path, tile_name, tile_format);
  let info = &archive.metadata;

  metadata.minzoom = Some(u32::from(header.min_zoom));
  metadata.maxzoom = Some(u32::from(header.max_zoom));
  let [west, south, east, north] = header.bounds;
  // an all-zero box is what writers leave behind when they did not compute one
  if east > west && north > south {
    metadata.bounds = Some(vec![west, south, east, north]);
  }
  if header.center[0] != 0.0 || header.center[1] != 0.0 {
    metadata.center = Some(header.center.to_vec());
  }
  metadata.name = json_string(info, "name");
  metadata.version = json_string(info, "version");
  metadata.description = json_string(info, "description");
  metadata.attribution = json_string(info, "attribution");
  metadata.layer_type = json_string(info, "type");
  if info.is_object() {
    metadata.json = Some(info.clone());
  }
  Ok(metadata)
}

fn get_mbtiles_details(path: &Path, tile_name: &str) -> Result<TileMeta> {
  let manager = SqliteConnectionManager::file(path).with_flags(OpenFlags::SQLITE_OPEN_READ_ONLY);
  let connection_pool = match r2d2::Pool::new(manager) {
    Ok(connection_pool) => connection_pool,
    Err(err) => return Err(Error::Pool(err)),
  };

  let connection = connection_pool.get().map_err(Error::Pool)?;

  // 'tiles', 'metadata' tables or views must be present
  let query = r#"SELECT count(*) FROM sqlite_master WHERE name IN ('tiles', 'metadata')"#;
  let mut statement = match connection.prepare(query) {
    Ok(s) => s,
    Err(err) => return Err(Error::DBConnection(err)),
  };
  match statement.query_row([], |row| Ok(row.get::<_, i8>(0).unwrap_or(0))) {
    Ok(count) => {
      if count < 2 {
        return Err(Error::MissingTable(String::from(tile_name)));
      }
    }
    Err(err) => return Err(Error::DBConnection(err)),
  };
  drop(statement);

  let tile_format = match get_data_format_via_query(tile_name, &connection, "tile") {
    Ok(tile_format) => match tile_format {
      DataFormat::Unknown => DataFormat::Pbf,
      DataFormat::Gzip => DataFormat::Pbf, // GZIP masks PBF format too
      _ => tile_format,
    },
    Err(err) => return Err(err),
  };
  // Sniffing cannot tell a broken MVT from a broken MLT, so the `format` metadata entry gets the
  // final say between the two vector formats. Image formats have magic bytes and stay as detected.
  let mut declared_format: Option<DataFormat> = None;

  let mut metadata = blank_meta(
    Backend::Mbtiles(connection_pool.clone()),
    path,
    tile_name,
    tile_format,
  );

  let mut statement = connection
    .prepare(r#"SELECT name, value FROM metadata WHERE value IS NOT NULL"#)
    .map_err(Error::DBConnection)?;
  let mut metadata_rows = statement.query([]).map_err(Error::DBConnection)?;

  while let Some(row) = metadata_rows.next().map_err(Error::DBConnection)? {
    let label: String = match row.get(0) {
      Ok(label) => label,
      Err(_) => continue,
    };
    // some writers store numbers as numbers; read them as the text the rest of this expects
    let value: String = match row.get_ref(1) {
      Ok(rusqlite::types::ValueRef::Text(text)) => String::from_utf8_lossy(text).into_owned(),
      Ok(rusqlite::types::ValueRef::Integer(number)) => number.to_string(),
      Ok(rusqlite::types::ValueRef::Real(number)) => number.to_string(),
      _ => continue,
    };
    match label.as_ref() {
      "name" => metadata.name = Some(value),
      "version" => metadata.version = Some(value),
      "bounds" => metadata.bounds = Some(value.split(',').filter_map(|s| s.parse().ok()).collect()),
      "center" => metadata.center = Some(value.split(',').filter_map(|s| s.parse().ok()).collect()),
      "minzoom" => {
        metadata.minzoom = Some(match value.parse::<u32>() {
          Ok(value) => value,
          Err(_) => 0,
        })
      }
      "maxzoom" => {
        metadata.maxzoom = Some(match value.parse::<u32>() {
          Ok(value) => value,
          Err(_) => 19,
        })
      }
      "format" => declared_format = Some(DataFormat::new(&value)),
      "description" => metadata.description = Some(value),
      "attribution" => metadata.attribution = Some(value),
      "type" => metadata.layer_type = Some(value),
      "legend" => metadata.legend = Some(value),
      "template" => metadata.template = Some(value),
      // a malformed blob only costs the layer list, not the whole file
      "json" => metadata.json = serde_json::from_str(&value).ok(),
      _ => (),
    }
  }
  if let Some(declared) = declared_format {
    if declared.is_vector() && metadata.tile_format.is_vector() {
      metadata.tile_format = declared;
    }
  }

  Ok(metadata)
}

/// One tile from an MBTiles file, addressed by its stored (TMS) row.
pub fn get_tile_data(connection: &Connection, z: u32, x: u32, y: u32) -> Result<Option<Vec<u8>>> {
  let mut statement = connection
    .prepare_cached(
      r#"SELECT tile_data
                 FROM tiles
                WHERE zoom_level = ?1
                  AND tile_column = ?2
                  AND tile_row = ?3
            "#,
    )
    .map_err(Error::DBConnection)?;
  match statement.query_row(params![z, x, y], |row| row.get::<_, Vec<u8>>(0)) {
    Ok(data) => Ok(Some(data)),
    Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
    Err(err) => Err(Error::DBConnection(err)),
  }
}

/// Vector tile bytes ready to serve: inflated when stored gzipped or zlibbed.
pub fn inflate_vector(data: Vec<u8>) -> Result<Vec<u8>> {
  let format = get_data_format(&data);
  decode(data, format)
}

#[cfg(test)]
mod tests {
  use super::*;

  fn temp_file(name: &str, bytes: &[u8]) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("mbview-tiles-test-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join(name);
    std::fs::write(&path, bytes).unwrap();
    path
  }

  #[test]
  fn opens_a_pmtiles_archive() {
    let path = temp_file("archive.pmtiles", &crate::pmtiles::tests::build_archive());
    let meta = get_tile_details(&path).unwrap();
    assert_eq!(meta.tile_format, DataFormat::Png);
    assert_eq!(meta.name.as_deref(), Some("test"));
    assert_eq!(meta.minzoom, Some(0));
    assert_eq!(meta.maxzoom, Some(3));
    assert_eq!(meta.bounds, Some(vec![-180.0, -85.0, 180.0, 85.0]));
    assert!(meta.json.as_ref().unwrap().get("vector_layers").is_some());
    // XYZ in, no row flipping for PMTiles
    assert_eq!(meta.get_tile(1, 0, 1).unwrap().unwrap(), b"tile-b");
    assert_eq!(meta.get_tile(3, 5, 6).unwrap().unwrap(), b"tile-deep");
    assert!(meta.get_tile(3, 0, 1).unwrap().is_none());
  }

  #[test]
  fn opens_an_mbtiles_file_and_flips_rows() {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../resources/world_cities.mbtiles");
    let meta = get_tile_details(&path).unwrap();
    assert!(meta.tile_format.is_vector());
    // z0 has a single tile, stored at TMS row 0 which is XYZ row 0 as well
    let tile = meta.get_tile(0, 0, 0).unwrap();
    assert!(tile.is_some());
    // an address outside the pyramid is just absent
    assert!(meta.get_tile(0, 0, 5).unwrap().is_none());
  }

  #[test]
  fn files_that_are_neither_format_are_rejected_not_panicked_on() {
    let path = temp_file("junk.pmtiles", &[7u8; 300]);
    let err = get_tile_details(&path).unwrap_err();
    assert!(err.to_string().contains("Not an MBTiles or PMTiles file"));
    let path = temp_file("empty.mbtiles", &[]);
    assert!(get_tile_details(&path).is_err());
    assert!(get_tile_details(Path::new("/nonexistent/file.pmtiles")).is_err());
  }
}
