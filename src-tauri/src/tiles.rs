// use parking_lot::{MappedMutexGuard, Mutex, MutexGuard};
use std::collections::HashMap;
use std::ffi::OsStr;
use std::path::{Path, PathBuf};
// use std::sync::Arc;
use hotwatch::{Event, Hotwatch};
use std::sync::Mutex;

use r2d2_sqlite::SqliteConnectionManager;
use rusqlite::{params, OpenFlags};

use serde::{Deserialize, Serialize};
use serde_json::Value as JSONValue;

use crate::errors::{Error, Result};

use crate::utils::{get_data_format, DataFormat};

pub type Connection = r2d2::PooledConnection<SqliteConnectionManager>;

#[derive(Clone, Debug)]
pub struct TileMeta {
  pub connection_pool: r2d2::Pool<SqliteConnectionManager>,
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
  if has_tilesetsdata(key) {
    get_tilesetsdata(key).data.clone()
  } else {
    None
  }
}

pub fn has_tilesetsdata(key: &String) -> bool {
  TILESET_MAP.lock().unwrap().contains_key(key)
}

fn get_tilesetsdata(key: &String) -> TilesetsData {
  TILESET_MAP.lock().unwrap().get_mut(key).unwrap().clone()
}
pub fn get_path(key: &String) -> Option<PathBuf> {
  if has_tilesetsdata(key) {
    Some(get_tilesetsdata(key).path.clone())
  } else {
    None
  }
}
pub fn set_mbtiles(key: &String, path: PathBuf, callback: Box<dyn CloneableFn>) {
  // println!("set_mbtiles {}", key);
  if has_tilesetsdata(key) {
    let mut hash_data = TILESET_MAP.lock().unwrap();
    let mut data = hash_data.get_mut(key).unwrap();
    match WATCHER.lock().unwrap().unwatch(&data.path) {
      Ok(_) => {}
      Err(err) => println!("error unwatch {}", err),
    }
    // println!("set_mbtiles {}", path.clone().to_str().unwrap());
    data.path = path.clone();
    data.data = match get_tile_details(&data.path) {
      Ok(tile_meta) => Some(tile_meta),
      Err(_) => None,
    };
    data.callback = callback;

    let watch_key = key.clone();
    WATCHER
      .lock()
      .unwrap()
      .watch(&path.clone(), move |event: Event| {
        // println!("mbtiles changed {}", path.clone().to_str().unwrap());
        if let Event::Write(_) = event {
          reload(&watch_key);
        }
      })
      .unwrap();
  } else {
    let data = TilesetsData {
      data: match get_tile_details(&path) {
        Ok(tile_meta) => Some(tile_meta),
        Err(_) => None,
      },
      path: path.clone(),
      callback: callback,
    };
    let mut hash_data = TILESET_MAP.lock().unwrap();
    hash_data.insert(key.clone(), data);
    let watch_key = key.clone();
    // println!("set_mbtiles {}", path.clone().to_str().unwrap());
    WATCHER
      .lock()
      .unwrap()
      .watch(&path.clone(), move |event: Event| {
        if let Event::Write(_) = event {
          reload(&watch_key);
        }
      })
      .unwrap();
  }
}

pub fn reload(key: &String) {
  // println!("reload {}", key);
  if has_tilesetsdata(key) {
    // println!("has_tilesetsdata {}", key);
    let mut hash_data = TILESET_MAP.lock().unwrap();
    let mut data = hash_data.get_mut(key).unwrap();
    data.data = match get_tile_details(&data.path) {
      Ok(tile_meta) => Some(tile_meta),
      Err(_) => None,
    };
    (data.callback)(key.clone());
  }
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
      Ok(get_data_format(&row.get::<_, Vec<u8>>(0).unwrap()))
    })
    .unwrap_or(DataFormat::Unknown);
  Ok(data_format)
}

pub fn get_tile_details(path: &Path) -> Result<TileMeta> {
  let manager = SqliteConnectionManager::file(path).with_flags(OpenFlags::SQLITE_OPEN_READ_ONLY);
  let connection_pool = match r2d2::Pool::new(manager) {
    Ok(connection_pool) => connection_pool,
    Err(err) => return Err(Error::Pool(err)),
  };
  let tile_name = path.file_name().and_then(OsStr::to_str).unwrap();

  let connection = connection_pool.get().unwrap();

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

  let tile_format = match get_data_format_via_query(tile_name, &connection, "tile") {
    Ok(tile_format) => match tile_format {
      DataFormat::Unknown => return Err(Error::UnknownTileFormat(String::from(tile_name))),
      DataFormat::Gzip => DataFormat::Pbf, // GZIP masks PBF format too
      _ => tile_format,
    },
    Err(err) => return Err(err),
  };

  let mut metadata = TileMeta {
    connection_pool,
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
    minzoom: None,
    maxzoom: None,
    description: None,
    attribution: None,
    layer_type: None,
    legend: None,
    template: None,
    json: None,
  };

  let mut statement = connection
    .prepare(r#"SELECT name, value FROM metadata WHERE value IS NOT ''"#)
    .unwrap();
  let mut metadata_rows = statement.query([]).unwrap();

  while let Some(row) = metadata_rows.next().unwrap() {
    let label: String = row.get(0).unwrap();
    let value: String = row.get(1).unwrap();
    match label.as_ref() {
      "name" => metadata.name = Some(value),
      "version" => metadata.version = Some(value),
      "bounds" => metadata.bounds = Some(value.split(',').filter_map(|s| s.parse().ok()).collect()),
      "center" => metadata.center = Some(value.split(',').filter_map(|s| s.parse().ok()).collect()),
      "minzoom" => metadata.minzoom = Some(value.parse().unwrap()),
      "maxzoom" => metadata.maxzoom = Some(value.parse().unwrap()),
      "description" => metadata.description = Some(value),
      "attribution" => metadata.attribution = Some(value),
      "type" => metadata.layer_type = Some(value),
      "legend" => metadata.legend = Some(value),
      "template" => metadata.template = Some(value),
      "json" => metadata.json = Some(serde_json::from_str(&value).unwrap()),
      _ => (),
    }
  }

  Ok(metadata)
}

// pub fn create_tilesets() -> Tilesets {
//   return Tilesets::new();
// }

pub fn get_tile_data(connection: &Connection, z: u32, x: u32, y: u32) -> Result<Vec<u8>> {
  let mut statement = connection
    .prepare(
      r#"SELECT tile_data
                 FROM tiles
                WHERE zoom_level = ?1
                  AND tile_column = ?2
                  AND tile_row = ?3
            "#,
    )
    .unwrap(); // TODO handle error
  match statement.query_row(params![z, x, y], |row| Ok(row.get(0).unwrap())) {
    Ok(data) => Ok(data),
    Err(err) => Err(Error::DBConnection(err)),
  }
}
