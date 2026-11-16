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
use crate::remote;
use std::io::Read;
use std::sync::Arc;

use crate::utils::{decode, get_data_format, get_tile_format, DataFormat};

pub type Connection = r2d2::PooledConnection<SqliteConnectionManager>;

/// Where the tiles of an open file actually live.
#[derive(Clone, Debug)]
pub enum Backend {
  Mbtiles(r2d2::Pool<SqliteConnectionManager>),
  Pmtiles(Arc<Pmtiles>),
  /// A web server handing out one tile per request.
  Remote(Arc<RemoteTiles>),
}

/// `{z}/{x}/{y}` URL templates, usually from a TileJSON.
#[derive(Debug)]
pub struct RemoteTiles {
  templates: Vec<String>,
  /// rows counted from the bottom, as a TileJSON with `"scheme": "tms"` has them
  tms: bool,
}

impl RemoteTiles {
  fn url(&self, z: u32, x: u32, y: u32) -> Option<String> {
    if self.templates.is_empty() {
      return None;
    }
    // spread the load over the servers of a TileJSON the way every client does
    let template = &self.templates[(z as usize + x as usize + y as usize) % self.templates.len()];
    let flipped = 1u32.checked_shl(z)?.checked_sub(1 + y)?;
    let row = if self.tms { flipped } else { y };
    Some(
      template
        .replace("{z}", &z.to_string())
        .replace("{x}", &x.to_string())
        .replace("{y}", &row.to_string())
        .replace("{-y}", &flipped.to_string()),
    )
  }
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
  /// Whether getting a tile means a network round trip, which must not block the async server.
  pub fn is_remote(&self) -> bool {
    match &self.backend {
      Backend::Mbtiles(_) => false,
      Backend::Pmtiles(archive) => archive.is_remote(),
      Backend::Remote(_) => true,
    }
  }

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
      Backend::Remote(remote_tiles) => match remote_tiles.url(z, x, y) {
        Some(url) => remote::get(&url, remote::MAX_TILE),
        None => Ok(None),
      },
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

  let remote = remote::is_remote(&path.to_string_lossy());
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
  // there is nothing on disk to watch for a web address
  if let Some(previous) = previous.filter(|previous| !remote::is_remote(&previous.to_string_lossy())) {
    if let Err(err) = lock_watcher().unwatch(&previous) {
      println!("error unwatch {}", err);
    }
  }
  if !remote {
    watch_file(key, &path);
  }
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
    // an empty blob has no magic bytes to read: look for a tile that has some
    "tile" => r#"SELECT tile_data FROM tiles WHERE length(tile_data) > 0 LIMIT 1"#,
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
  let location = path.to_string_lossy();
  if remote::is_remote(&location) {
    return get_remote_details(location.trim());
  }
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
  pmtiles_meta(Arc::new(Pmtiles::open(path)?), path, tile_name)
}

fn pmtiles_meta(archive: Arc<Pmtiles>, path: &Path, tile_name: &str) -> Result<TileMeta> {
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

/// A short name for a web address: the last thing in its path, or its host.
fn remote_name(url: &str) -> String {
  let path = remote::path_part(url).trim_end_matches('/');
  let rest = path.split("://").nth(1).unwrap_or(path);
  let mut parts = rest.split('/');
  let host = parts.next().unwrap_or(rest);
  match parts.filter(|part| !part.is_empty() && !part.contains('{')).last() {
    Some(last) => last.to_string(),
    None => host.to_string(),
  }
}

fn json_u32(value: &JSONValue, key: &str) -> Option<u32> {
  match value.get(key)? {
    JSONValue::Number(number) => number.as_f64().map(|n| n as u32),
    JSONValue::String(text) => text.trim().parse::<f64>().ok().map(|n| n as u32),
    _ => None,
  }
}

/// A list of numbers, written as a JSON array or as "1,2,3,4".
fn json_numbers(value: &JSONValue, key: &str) -> Option<Vec<f64>> {
  let list: Vec<f64> = match value.get(key)? {
    JSONValue::Array(items) => items.iter().filter_map(|item| item.as_f64()).collect(),
    JSONValue::String(text) => text.split(',').filter_map(|part| part.trim().parse().ok()).collect(),
    _ => return None,
  };
  Some(list)
}

fn read_varint(buf: &[u8], pos: &mut usize) -> Option<u64> {
  let mut value = 0u64;
  for i in 0..10 {
    let byte = *buf.get(*pos)?;
    *pos += 1;
    value |= u64::from(byte & 0x7f) << (i * 7);
    if byte & 0x80 == 0 {
      return Some(value);
    }
  }
  None
}

/// The length-delimited fields of a protobuf message, as `(field number, bytes)`.
fn proto_blobs(buf: &[u8]) -> Vec<(u64, &[u8])> {
  let mut out = Vec::new();
  let mut pos = 0usize;
  while pos < buf.len() {
    let key = match read_varint(buf, &mut pos) {
      Some(key) => key,
      None => break,
    };
    match key & 7 {
      0 => {
        if read_varint(buf, &mut pos).is_none() {
          break;
        }
      }
      1 => pos += 8,
      5 => pos += 4,
      2 => {
        let len = match read_varint(buf, &mut pos) {
          Some(len) => len as usize,
          None => break,
        };
        match pos.checked_add(len) {
          Some(end) if end <= buf.len() => {
            out.push((key >> 3, &buf[pos..end]));
            pos = end;
          }
          _ => break,
        }
      }
      _ => break,
    }
  }
  out
}

/// Names of the layers in a Mapbox Vector Tile: field 3 of a tile is a layer, whose field 1 is its name.
fn mvt_layer_names(tile: &[u8]) -> Vec<String> {
  proto_blobs(tile)
    .into_iter()
    .filter(|(field, _)| *field == 3)
    .filter_map(|(_, layer)| {
      proto_blobs(layer)
        .into_iter()
        .find(|(field, _)| *field == 1)
        .and_then(|(_, name)| String::from_utf8(name.to_vec()).ok())
    })
    .collect()
}

/// How far down the pyramid a bare template is explored.
const SNIFF_MAX_ZOOM: u32 = 16;

/// Layers and deepest zoom of a bare vector template, which has no TileJSON to say what is inside.
///
/// One tile is not enough: a world tile only holds the layers that exist at low zoom (water,
/// boundaries, places), while roads, buildings and POIs appear deeper. So this starts at the first
/// tile that answers and keeps stepping into the busiest of its four children, merging the layer
/// names of everything it meets, until a level has no tile.
fn sniff_vector_layers(meta: &TileMeta) -> (Vec<String>, u32) {
  let fetch = |z: u32, x: u32, y: u32| {
    meta
      .get_tile(z, x, y)
      .ok()
      .flatten()
      .and_then(|tile| inflate_vector(tile).ok())
      // some servers answer 200 with nothing past their last zoom instead of a 404
      .filter(|tile| !tile.is_empty())
  };
  let mut names: Vec<String> = Vec::new();
  let mut add = |tile: &[u8]| {
    for name in mvt_layer_names(tile) {
      if !names.contains(&name) {
        names.push(name);
      }
    }
  };

  // a regional tileset has nothing at 0/0/0: look a little further for somewhere it answers
  let mut at = None;
  'seed: for z in 0..=2u32 {
    for x in 0..(1u32 << z) {
      for y in 0..(1u32 << z) {
        if let Some(tile) = fetch(z, x, y) {
          add(&tile);
          at = Some((z, x, y));
          break 'seed;
        }
      }
    }
  }
  let (mut z, mut x, mut y) = match at {
    Some(at) => at,
    None => return (names, 0),
  };
  let mut deepest = z;
  while z < SNIFF_MAX_ZOOM {
    let mut busiest: Option<(usize, u32, u32)> = None;
    for (dx, dy) in [(0, 0), (1, 0), (0, 1), (1, 1)] {
      let (cx, cy) = (x * 2 + dx, y * 2 + dy);
      if let Some(tile) = fetch(z + 1, cx, cy) {
        add(&tile);
        if busiest.map_or(true, |(size, _, _)| tile.len() > size) {
          busiest = Some((tile.len(), cx, cy));
        }
      }
    }
    match busiest {
      Some((_, cx, cy)) => {
        z += 1;
        x = cx;
        y = cy;
        deepest = z;
      }
      None => break,
    }
  }
  (names, deepest)
}

fn get_remote_details(url: &str) -> Result<TileMeta> {
  let name = remote_name(url);
  let path = Path::new(url);
  if remote::path_part(url).to_ascii_lowercase().ends_with(".pmtiles") {
    return pmtiles_meta(Arc::new(Pmtiles::open_url(url)?), path, &name);
  }
  if url.contains("{z}") {
    return remote_details(url, &name, vec![url.to_string()], false, JSONValue::Null);
  }
  let body = remote::get(url, remote::MAX_JSON)?
    .ok_or_else(|| Error::Remote(format!("{}: not found", url)))?;
  // an archive served without the usual extension
  if body.starts_with(pmtiles::MAGIC) {
    return pmtiles_meta(Arc::new(Pmtiles::open_url(url)?), path, &name);
  }
  let json: JSONValue = serde_json::from_slice(&body)
    .map_err(|_| Error::Remote(format!("{}: neither a TileJSON nor a PMTiles archive", url)))?;
  let templates: Vec<String> = json
    .get("tiles")
    .and_then(|tiles| tiles.as_array())
    .map(|tiles| {
      tiles
        .iter()
        .filter_map(|tile| tile.as_str())
        .filter(|tile| remote::is_remote(tile))
        .map(String::from)
        .collect()
    })
    .unwrap_or_default();
  if templates.is_empty() {
    return Err(Error::Remote(format!("{}: the TileJSON lists no tile URLs", url)));
  }
  let tms = json.get("scheme").and_then(|scheme| scheme.as_str()) == Some("tms");
  remote_details(url, &name, templates, tms, json)
}

/// A source made of URL templates, with whatever its TileJSON (`Null` when it has none) says.
fn remote_details(
  url: &str,
  name: &str,
  templates: Vec<String>,
  tms: bool,
  info: JSONValue,
) -> Result<TileMeta> {
  let extension = remote::path_part(&templates[0])
    .rsplit('.')
    .next()
    .map(|ext| ext.to_ascii_lowercase())
    .unwrap_or_default();
  let has_layers = info.get("vector_layers").is_some() || info.get("Layer").is_some();
  let declared = json_string(&info, "format").map(|format| DataFormat::new(&format.to_ascii_lowercase()));
  let mut tile_format = match declared {
    Some(format) if format != DataFormat::Unknown => format,
    _ => DataFormat::new(&extension),
  };
  if tile_format == DataFormat::Unknown || (has_layers && !tile_format.is_vector()) {
    tile_format = DataFormat::Pbf;
  }

  let mut meta = blank_meta(
    Backend::Remote(Arc::new(RemoteTiles { templates, tms })),
    Path::new(url),
    name,
    tile_format,
  );
  meta.minzoom = json_u32(&info, "minzoom").or(Some(0));
  meta.maxzoom = json_u32(&info, "maxzoom").or(Some(22));
  meta.bounds = json_numbers(&info, "bounds").filter(|bounds| bounds.len() == 4);
  meta.center = json_numbers(&info, "center").filter(|center| center.len() >= 2);
  meta.name = json_string(&info, "name");
  meta.version = json_string(&info, "version");
  meta.description = json_string(&info, "description");
  meta.attribution = json_string(&info, "attribution");
  meta.layer_type = json_string(&info, "type");
  if info.is_object() {
    meta.json = Some(info);
  }

  if tile_format.is_vector() && !has_layers {
    // a bare template: without a layer list the viewer would take it for a raster
    if tile_format == DataFormat::Mlt {
      return Err(Error::Remote(format!(
        "{}: MLT tiles need a TileJSON that lists their layers",
        url
      )));
    }
    let (layers, deepest) = sniff_vector_layers(&meta);
    if layers.is_empty() {
      return Err(Error::Remote(format!(
        "{}: no vector tile found to read the layers from, give a TileJSON instead",
        url
      )));
    }
    // asking for tiles deeper than the server has would only draw nothing
    meta.maxzoom = Some(deepest);
    meta.json = Some(serde_json::json!({
      "vector_layers": layers.iter().map(|id| serde_json::json!({ "id": id })).collect::<Vec<_>>(),
    }));
  }
  Ok(meta)
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
    } else if declared.is_image() && metadata.tile_format.is_vector() {
      // sniffing falls back to a vector format when it recognises nothing: an archive that
      // says it holds images and has no vector layers is more likely a raster we could not read
      let has_layers = metadata
        .json
        .as_ref()
        .map_or(false, |json| json.get("vector_layers").is_some() || json.get("Layer").is_some());
      if !has_layers {
        metadata.tile_format = declared;
      }
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
pub(crate) mod tests {
  use super::*;
  use std::io::Write;

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

  /// Builds an MBTiles file with one table row per `(zoom, column, row, blob)`.
  fn raster_mbtiles(name: &str, format: &str, rows: &[(u32, u32, u32, Vec<u8>)]) -> PathBuf {
    let path = temp_file(name, &[]);
    std::fs::remove_file(&path).ok();
    let db = rusqlite::Connection::open(&path).unwrap();
    db.execute_batch(
      "CREATE TABLE metadata (name TEXT, value TEXT);
       CREATE TABLE tiles (zoom_level INTEGER, tile_column INTEGER, tile_row INTEGER, tile_data BLOB);",
    )
    .unwrap();
    db.execute("INSERT INTO metadata VALUES ('format', ?1)", [format]).unwrap();
    for (z, x, y, data) in rows {
      db.execute("INSERT INTO tiles VALUES (?1, ?2, ?3, ?4)", params![z, x, y, data]).unwrap();
    }
    path
  }

  fn webp_tile() -> Vec<u8> {
    let mut tile = b"RIFF".to_vec();
    tile.extend_from_slice(&[0x10, 0, 0, 0]);
    tile.extend_from_slice(b"WEBPVP8 ");
    tile.extend_from_slice(&[0u8; 16]);
    tile
  }

  #[test]
  fn a_webp_mbtiles_is_a_raster() {
    let path = raster_mbtiles("webp.mbtiles", "webp", &[(0, 0, 0, webp_tile())]);
    assert_eq!(get_tile_details(&path).unwrap().tile_format, DataFormat::Webp);
  }

  #[test]
  fn an_empty_first_tile_does_not_hide_the_image_format() {
    let path = raster_mbtiles(
      "webp-empty-first.mbtiles",
      "webp",
      &[(0, 0, 0, Vec::new()), (1, 0, 0, webp_tile())],
    );
    assert_eq!(get_tile_details(&path).unwrap().tile_format, DataFormat::Webp);
  }

  #[test]
  fn compressed_image_tiles_are_still_images() {
    use flate2::write::GzEncoder;
    use flate2::Compression;
    let mut gz = GzEncoder::new(Vec::new(), Compression::default());
    gz.write_all(&webp_tile()).unwrap();
    let path = raster_mbtiles("webp-gz.mbtiles", "webp", &[(0, 0, 0, gz.finish().unwrap())]);
    assert_eq!(get_tile_details(&path).unwrap().tile_format, DataFormat::Webp);
  }

  #[test]
  fn the_declared_image_format_wins_over_an_unreadable_first_tile() {
    // not a recognisable image, but the file says it holds webp and lists no vector layers
    let path = raster_mbtiles("webp-odd.mbtiles", "webp", &[(0, 0, 0, vec![1, 2, 3, 4, 5, 6, 7, 8])]);
    assert_eq!(get_tile_details(&path).unwrap().tile_format, DataFormat::Webp);
  }

  /// A web server just big enough for these tests: one archive that honours `Range`, a TileJSON,
  /// and a single vector tile at 0/0/0.
  pub(crate) fn spawn_web_server() -> u16 {
    use std::io::{BufRead, BufReader};
    use std::net::TcpListener;
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port();
    let archive = crate::pmtiles::tests::build_archive();
    std::thread::spawn(move || {
      for stream in listener.incoming() {
        let mut stream = match stream {
          Ok(stream) => stream,
          Err(_) => continue,
        };
        let mut reader = BufReader::new(stream.try_clone().unwrap());
        let mut line = String::new();
        reader.read_line(&mut line).unwrap();
        let path = line.split(' ').nth(1).unwrap_or("/").to_string();
        let mut range: Option<(usize, usize)> = None;
        loop {
          let mut header = String::new();
          reader.read_line(&mut header).unwrap();
          if header.trim().is_empty() {
            break;
          }
          if let Some(value) = header.to_ascii_lowercase().strip_prefix("range: bytes=") {
            let mut parts = value.trim().split('-');
            let start = parts.next().and_then(|n| n.parse().ok()).unwrap_or(0);
            let end = parts.next().and_then(|n| n.parse().ok()).unwrap_or(0);
            range = Some((start, end));
          }
        }
        let make = |name: &[u8]| {
          let layer = [&[0x0a, name.len() as u8][..], name].concat();
          [&[0x1a, layer.len() as u8][..], &layer].concat()
        };
        let mvt = make(b"water");
        let (status, body) = match path.as_str() {
          "/a.pmtiles" => match range {
            Some((start, end)) => ("206 Partial Content", archive[start..=end.min(archive.len() - 1)].to_vec()),
            None => ("200 OK", archive.clone()),
          },
          // the same archive, but a server that ignores ranges
          "/norange.pmtiles" => ("200 OK", archive.clone()),
          "/tilejson" => (
            "200 OK",
            format!(
              r#"{{"tilejson":"3.0.0","tiles":["http://127.0.0.1:{0}/t/{{z}}/{{x}}/{{y}}.pbf"],"vector_layers":[{{"id":"water"}}],"minzoom":"0","maxzoom":"14","bounds":[1,2,3,4]}}"#,
              port
            )
            .into_bytes(),
          ),
          "/t/0/0/0.pbf" => ("200 OK", mvt),
          // layers that only exist deeper down, as roads and buildings do
          "/t/1/1/0.pbf" => ("200 OK", make(b"road")),
          "/t/2/2/1.pbf" => ("200 OK", make(b"building")),
          // an empty 200 below the last real level must not count as data
          "/t/3/4/2.pbf" => ("200 OK", Vec::new()),
          _ => ("404 Not Found", Vec::new()),
        };
        let head = format!("HTTP/1.1 {}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n", status, body.len());
        let _ = stream.write_all(head.as_bytes());
        let _ = stream.write_all(&body);
      }
    });
    port
  }

  #[test]
  fn reads_a_pmtiles_archive_over_http_ranges() {
    let port = spawn_web_server();
    let meta = get_tile_details(Path::new(&format!("http://127.0.0.1:{}/a.pmtiles", port))).unwrap();
    assert!(meta.is_remote());
    assert_eq!(meta.tile_format, DataFormat::Png);
    assert_eq!(meta.maxzoom, Some(3));
    assert_eq!(meta.get_tile(1, 0, 1).unwrap().unwrap(), b"tile-b");
    // through a leaf directory, which is a second range request
    assert_eq!(meta.get_tile(3, 5, 6).unwrap().unwrap(), b"tile-deep");
    assert!(meta.get_tile(1, 1, 1).unwrap().is_none());
  }

  #[test]
  fn a_server_without_range_support_is_refused() {
    let port = spawn_web_server();
    let err = get_tile_details(Path::new(&format!("http://127.0.0.1:{}/norange.pmtiles", port))).unwrap_err();
    assert!(err.to_string().contains("range"), "{}", err);
  }

  #[test]
  fn follows_a_tilejson_to_its_tiles() {
    let port = spawn_web_server();
    let meta = get_tile_details(Path::new(&format!("http://127.0.0.1:{}/tilejson", port))).unwrap();
    assert!(meta.tile_format.is_vector());
    assert_eq!(meta.maxzoom, Some(14));
    assert_eq!(meta.bounds, Some(vec![1.0, 2.0, 3.0, 4.0]));
    assert!(meta.get_tile(0, 0, 0).unwrap().is_some());
    assert!(meta.get_tile(1, 0, 0).unwrap().is_none());
  }

  #[test]
  fn reads_the_layers_of_a_bare_vector_template() {
    let port = spawn_web_server();
    let url = format!("http://127.0.0.1:{}/t/{{z}}/{{x}}/{{y}}.pbf", port);
    let meta = get_tile_details(Path::new(&url)).unwrap();
    let layers: Vec<String> = meta.json.as_ref().unwrap()["vector_layers"]
      .as_array()
      .unwrap()
      .iter()
      .map(|layer| layer["id"].as_str().unwrap().to_string())
      .collect();
    // not just what the world tile holds: what the tiles below it add, down to where they stop
    assert_eq!(layers, vec!["water", "road", "building"]);
    assert_eq!(meta.maxzoom, Some(2));
  }

  #[test]
  fn template_rows_can_be_flipped() {
    let tiles = RemoteTiles { templates: vec![String::from("https://a/{z}/{x}/{y}|{-y}")], tms: false };
    assert_eq!(tiles.url(2, 1, 0).unwrap(), "https://a/2/1/0|3");
    let tiles = RemoteTiles { templates: vec![String::from("https://a/{z}/{x}/{y}")], tms: true };
    assert_eq!(tiles.url(2, 1, 0).unwrap(), "https://a/2/1/3");
  }

  /// The real thing: `cargo test openfreemap -- --ignored` (needs the network).
  #[test]
  #[ignore]
  fn openfreemap_planet() {
    let meta = get_tile_details(Path::new("https://tiles.openfreemap.org/planet")).unwrap();
    assert!(meta.tile_format.is_vector());
    assert!(meta.json.as_ref().unwrap()["vector_layers"].as_array().unwrap().len() > 5);
    let tile = meta.get_tile(2, 2, 1).unwrap().unwrap();
    assert!(!crate::tiles::inflate_vector(tile).unwrap().is_empty());
  }

  #[test]
  #[ignore]
  fn openfreemap_bare_template() {
    let meta = get_tile_details(Path::new("https://tiles.openfreemap.org/planet/latest/{z}/{x}/{y}.pbf")).unwrap();
    println!("{:?} maxzoom {:?}", meta.json.as_ref().unwrap()["vector_layers"], meta.maxzoom);
    assert!(meta.json.as_ref().unwrap()["vector_layers"].as_array().unwrap().len() > 10);
  }
}
