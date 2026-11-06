//! Minimal read-only PMTiles v3 reader.
//!
//! An archive is a 127 byte header, a root directory, optional leaf directories, a JSON metadata
//! blob and the tile data, all addressed by absolute offsets, so a tile lookup never has to read
//! more than a directory or two plus the tile itself. Tiles are keyed by a Hilbert curve id.
//! See <https://github.com/protomaps/PMTiles/blob/main/spec/v3/spec.md>.

use std::collections::HashMap;
use std::fs::File;
use std::io::Read;
use std::path::Path;
use std::sync::{Arc, Mutex};

use flate2::read::GzDecoder;
use serde_json::Value as JSONValue;

use crate::errors::{Error, Result};

pub const MAGIC: &[u8; 7] = b"PMTiles";
const HEADER_LEN: usize = 127;
/// Leaf directories are small but numerous on big archives; this bounds how many stay parsed.
const LEAF_CACHE_MAX: usize = 64;
/// Directories nest root -> leaf -> leaf at most; anything deeper is a corrupt (or hostile) file.
const MAX_DEPTH: usize = 4;
/// Refuse to inflate more than this for a single directory or metadata blob.
const MAX_INFLATED: u64 = 256 * 1024 * 1024;

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Compression {
  Unknown,
  None,
  Gzip,
  Brotli,
  Zstd,
}

impl Compression {
  fn from_u8(value: u8) -> Compression {
    match value {
      1 => Compression::None,
      2 => Compression::Gzip,
      3 => Compression::Brotli,
      4 => Compression::Zstd,
      _ => Compression::Unknown,
    }
  }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum TileType {
  Unknown,
  Mvt,
  Png,
  Jpeg,
  Webp,
  Avif,
  Mlt,
}

impl TileType {
  fn from_u8(value: u8) -> TileType {
    match value {
      1 => TileType::Mvt,
      2 => TileType::Png,
      3 => TileType::Jpeg,
      4 => TileType::Webp,
      5 => TileType::Avif,
      6 => TileType::Mlt,
      _ => TileType::Unknown,
    }
  }
}

#[derive(Clone, Debug)]
#[allow(dead_code)]
pub struct Header {
  pub root_offset: u64,
  pub root_length: u64,
  pub metadata_offset: u64,
  pub metadata_length: u64,
  pub leaf_offset: u64,
  pub leaf_length: u64,
  pub data_offset: u64,
  pub data_length: u64,
  pub internal_compression: Compression,
  pub tile_compression: Compression,
  pub tile_type: TileType,
  pub min_zoom: u8,
  pub max_zoom: u8,
  /// west, south, east, north
  pub bounds: [f64; 4],
  /// lon, lat, zoom
  pub center: [f64; 3],
}

fn u64_at(buf: &[u8], pos: usize) -> u64 {
  let mut bytes = [0u8; 8];
  bytes.copy_from_slice(&buf[pos..pos + 8]);
  u64::from_le_bytes(bytes)
}

fn i32_at(buf: &[u8], pos: usize) -> i32 {
  let mut bytes = [0u8; 4];
  bytes.copy_from_slice(&buf[pos..pos + 4]);
  i32::from_le_bytes(bytes)
}

impl Header {
  pub fn parse(buf: &[u8]) -> Result<Header> {
    if buf.len() < HEADER_LEN || &buf[0..7] != MAGIC {
      return Err(Error::InvalidPmtiles(String::from("missing PMTiles header")));
    }
    if buf[7] != 3 {
      return Err(Error::InvalidPmtiles(format!(
        "unsupported version {} (only version 3 is supported)",
        buf[7]
      )));
    }
    let e7 = |pos: usize| f64::from(i32_at(buf, pos)) / 10_000_000.0;
    Ok(Header {
      root_offset: u64_at(buf, 8),
      root_length: u64_at(buf, 16),
      metadata_offset: u64_at(buf, 24),
      metadata_length: u64_at(buf, 32),
      leaf_offset: u64_at(buf, 40),
      leaf_length: u64_at(buf, 48),
      data_offset: u64_at(buf, 56),
      data_length: u64_at(buf, 64),
      internal_compression: Compression::from_u8(buf[97]),
      tile_compression: Compression::from_u8(buf[98]),
      tile_type: TileType::from_u8(buf[99]),
      min_zoom: buf[100],
      max_zoom: buf[101],
      bounds: [e7(102), e7(106), e7(110), e7(114)],
      center: [e7(123), e7(119), f64::from(buf[118])],
    })
  }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Entry {
  pub tile_id: u64,
  pub offset: u64,
  pub length: u32,
  /// 0 means `offset`/`length` point at a leaf directory instead of a tile.
  pub run_length: u32,
}

/// Id of tile `z/x/y` along the Hilbert curve, zoom levels laid end to end.
pub fn zxy_to_tile_id(z: u8, x: u32, y: u32) -> Option<u64> {
  if z > 31 {
    return None;
  }
  let n: u64 = 1 << z;
  if u64::from(x) >= n || u64::from(y) >= n {
    return None;
  }
  let acc: u64 = ((1u64 << (u32::from(z) * 2)) - 1) / 3;
  let (mut tx, mut ty) = (u64::from(x), u64::from(y));
  let mut d: u64 = 0;
  let mut s = n / 2;
  while s > 0 {
    let rx = u64::from(tx & s > 0);
    let ry = u64::from(ty & s > 0);
    d += s * s * ((3 * rx) ^ ry);
    // rotate the quadrant so the curve stays continuous
    if ry == 0 {
      if rx == 1 {
        tx = n - 1 - tx;
        ty = n - 1 - ty;
      }
      std::mem::swap(&mut tx, &mut ty);
    }
    s /= 2;
  }
  Some(acc + d)
}

fn read_varint(buf: &[u8], pos: &mut usize) -> Result<u64> {
  let mut value = 0u64;
  for i in 0..10 {
    let byte = *buf
      .get(*pos)
      .ok_or_else(|| Error::InvalidPmtiles(String::from("truncated directory")))?;
    *pos += 1;
    value |= u64::from(byte & 0x7f) << (i * 7);
    if byte & 0x80 == 0 {
      return Ok(value);
    }
  }
  Err(Error::InvalidPmtiles(String::from("varint too long")))
}

pub fn parse_directory(buf: &[u8]) -> Result<Vec<Entry>> {
  let mut pos = 0usize;
  let count = read_varint(buf, &mut pos)? as usize;
  // every entry takes at least four bytes, which bounds what a corrupt count can make us allocate
  if count > buf.len() {
    return Err(Error::InvalidPmtiles(String::from("directory entry count")));
  }
  let mut entries = vec![
    Entry {
      tile_id: 0,
      offset: 0,
      length: 0,
      run_length: 0,
    };
    count
  ];
  let mut last_id = 0u64;
  for entry in entries.iter_mut() {
    last_id = last_id.wrapping_add(read_varint(buf, &mut pos)?);
    entry.tile_id = last_id;
  }
  for entry in entries.iter_mut() {
    entry.run_length = read_varint(buf, &mut pos)? as u32;
  }
  for entry in entries.iter_mut() {
    entry.length = read_varint(buf, &mut pos)? as u32;
  }
  for i in 0..entries.len() {
    let value = read_varint(buf, &mut pos)?;
    entries[i].offset = if value == 0 && i > 0 {
      entries[i - 1].offset + u64::from(entries[i - 1].length)
    } else {
      value.saturating_sub(1)
    };
  }
  Ok(entries)
}

/// What a lookup in one directory found.
enum Lookup {
  Tile(Entry),
  Leaf(Entry),
  Missing,
}

fn find_in_directory(entries: &[Entry], tile_id: u64) -> Lookup {
  // first entry whose id is greater than ours; the one before it is the only candidate
  let after = entries.partition_point(|entry| entry.tile_id <= tile_id);
  if after == 0 {
    return Lookup::Missing;
  }
  let entry = entries[after - 1];
  if entry.run_length == 0 {
    return Lookup::Leaf(entry);
  }
  if entry.tile_id == tile_id || tile_id < entry.tile_id + u64::from(entry.run_length) {
    return Lookup::Tile(entry);
  }
  Lookup::Missing
}

pub fn decompress(data: Vec<u8>, compression: Compression) -> Result<Vec<u8>> {
  match compression {
    Compression::None | Compression::Unknown => Ok(data),
    Compression::Gzip => {
      let mut out = Vec::new();
      GzDecoder::new(&data[..])
        .take(MAX_INFLATED)
        .read_to_end(&mut out)?;
      Ok(out)
    }
    Compression::Brotli => Err(Error::InvalidPmtiles(String::from(
      "brotli compression is not supported",
    ))),
    Compression::Zstd => Err(Error::InvalidPmtiles(String::from(
      "zstd compression is not supported",
    ))),
  }
}

#[cfg(unix)]
fn read_exact_at(file: &File, buf: &mut [u8], offset: u64) -> std::io::Result<()> {
  std::os::unix::fs::FileExt::read_exact_at(file, buf, offset)
}

#[cfg(windows)]
fn read_exact_at(file: &File, mut buf: &mut [u8], mut offset: u64) -> std::io::Result<()> {
  while !buf.is_empty() {
    match std::os::windows::fs::FileExt::seek_read(file, buf, offset) {
      Ok(0) => break,
      Ok(n) => {
        buf = &mut buf[n..];
        offset += n as u64;
      }
      Err(err) if err.kind() == std::io::ErrorKind::Interrupted => {}
      Err(err) => return Err(err),
    }
  }
  if buf.is_empty() {
    Ok(())
  } else {
    Err(std::io::ErrorKind::UnexpectedEof.into())
  }
}

pub struct Pmtiles {
  file: File,
  file_len: u64,
  pub header: Header,
  pub metadata: JSONValue,
  root: Vec<Entry>,
  leaves: Mutex<HashMap<u64, Arc<Vec<Entry>>>>,
}

impl std::fmt::Debug for Pmtiles {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    f.debug_struct("Pmtiles")
      .field("header", &self.header)
      .finish()
  }
}

impl Pmtiles {
  pub fn open(path: &Path) -> Result<Pmtiles> {
    let file = File::open(path)?;
    let file_len = file.metadata()?.len();
    let mut head = [0u8; HEADER_LEN];
    if file_len < HEADER_LEN as u64 {
      return Err(Error::InvalidPmtiles(String::from("file too small")));
    }
    read_exact_at(&file, &mut head, 0)?;
    let header = Header::parse(&head)?;

    // a tile compression we cannot undo would only fail later, tile by tile
    match (header.tile_compression, header.tile_type) {
      (Compression::Brotli, _) | (Compression::Zstd, _) => {
        return Err(Error::InvalidPmtiles(format!(
          "{:?} tile compression is not supported",
          header.tile_compression
        )))
      }
      _ => {}
    }

    let mut archive = Pmtiles {
      file,
      file_len,
      header,
      metadata: JSONValue::Null,
      root: Vec::new(),
      leaves: Mutex::new(HashMap::new()),
    };
    let root = archive.read_directory(archive.header.root_offset, archive.header.root_length)?;
    archive.root = root;
    // a missing or mangled metadata blob is not fatal: the header alone is enough to draw tiles
    archive.metadata = archive
      .read_range(archive.header.metadata_offset, archive.header.metadata_length)
      .and_then(|raw| decompress(raw, archive.header.internal_compression))
      .ok()
      .and_then(|raw| serde_json::from_slice(&raw).ok())
      .unwrap_or(JSONValue::Null);
    Ok(archive)
  }

  fn read_range(&self, offset: u64, length: u64) -> Result<Vec<u8>> {
    match offset.checked_add(length) {
      Some(end) if end <= self.file_len => {}
      _ => {
        return Err(Error::InvalidPmtiles(String::from(
          "section runs past the end of the file",
        )))
      }
    }
    let mut buf = vec![0u8; length as usize];
    read_exact_at(&self.file, &mut buf, offset)?;
    Ok(buf)
  }

  fn read_directory(&self, offset: u64, length: u64) -> Result<Vec<Entry>> {
    let raw = self.read_range(offset, length)?;
    parse_directory(&decompress(raw, self.header.internal_compression)?)
  }

  fn leaf(&self, offset: u64, length: u64) -> Result<Arc<Vec<Entry>>> {
    if let Some(entries) = self.leaves.lock().unwrap().get(&offset) {
      return Ok(entries.clone());
    }
    // read outside the lock so one slow leaf does not stall every other tile request
    let entries = Arc::new(self.read_directory(offset, length)?);
    let mut cache = self.leaves.lock().unwrap();
    if cache.len() >= LEAF_CACHE_MAX {
      cache.clear();
    }
    cache.insert(offset, entries.clone());
    Ok(entries)
  }

  /// Raw tile bytes as stored (still compressed with `header.tile_compression`), or `None` when
  /// the archive has no such tile.
  pub fn get_tile(&self, z: u8, x: u32, y: u32) -> Result<Option<Vec<u8>>> {
    let tile_id = match zxy_to_tile_id(z, x, y) {
      Some(id) => id,
      None => return Ok(None),
    };
    let mut current: Arc<Vec<Entry>> = Arc::new(Vec::new());
    for depth in 0..MAX_DEPTH {
      let found = if depth == 0 {
        find_in_directory(&self.root, tile_id)
      } else {
        find_in_directory(&current, tile_id)
      };
      match found {
        Lookup::Missing => return Ok(None),
        Lookup::Tile(entry) => {
          let offset = self.header.data_offset.checked_add(entry.offset);
          return match offset {
            Some(offset) => Ok(Some(self.read_range(offset, u64::from(entry.length))?)),
            None => Err(Error::InvalidPmtiles(String::from("tile offset overflow"))),
          };
        }
        Lookup::Leaf(entry) => {
          let offset = self
            .header
            .leaf_offset
            .checked_add(entry.offset)
            .ok_or_else(|| Error::InvalidPmtiles(String::from("leaf offset overflow")))?;
          current = self.leaf(offset, u64::from(entry.length))?;
        }
      }
    }
    Err(Error::InvalidPmtiles(String::from(
      "directories nested too deeply",
    )))
  }
}

#[cfg(test)]
pub(crate) mod tests {
  use super::*;
  use std::io::Write;

  fn varint(out: &mut Vec<u8>, mut value: u64) {
    loop {
      let byte = (value & 0x7f) as u8;
      value >>= 7;
      if value == 0 {
        out.push(byte);
        return;
      }
      out.push(byte | 0x80);
    }
  }

  /// Serialises entries the way the spec lays them out: columns, ids and offsets delta-coded.
  fn write_directory(entries: &[Entry]) -> Vec<u8> {
    let mut out = Vec::new();
    varint(&mut out, entries.len() as u64);
    let mut last = 0;
    for entry in entries {
      varint(&mut out, entry.tile_id - last);
      last = entry.tile_id;
    }
    for entry in entries {
      varint(&mut out, u64::from(entry.run_length));
    }
    for entry in entries {
      varint(&mut out, u64::from(entry.length));
    }
    for (i, entry) in entries.iter().enumerate() {
      if i > 0 && entry.offset == entries[i - 1].offset + u64::from(entries[i - 1].length) {
        varint(&mut out, 0);
      } else {
        varint(&mut out, entry.offset + 1);
      }
    }
    out
  }

  fn gzip(data: &[u8]) -> Vec<u8> {
    let mut encoder =
      flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::default());
    encoder.write_all(data).unwrap();
    encoder.finish().unwrap()
  }

  #[test]
  fn tile_ids_follow_the_hilbert_curve() {
    assert_eq!(zxy_to_tile_id(0, 0, 0), Some(0));
    assert_eq!(zxy_to_tile_id(1, 0, 0), Some(1));
    assert_eq!(zxy_to_tile_id(1, 0, 1), Some(2));
    assert_eq!(zxy_to_tile_id(1, 1, 1), Some(3));
    assert_eq!(zxy_to_tile_id(1, 1, 0), Some(4));
    assert_eq!(zxy_to_tile_id(2, 0, 0), Some(5));
    // out of range
    assert_eq!(zxy_to_tile_id(1, 2, 0), None);
    assert_eq!(zxy_to_tile_id(40, 0, 0), None);
  }

  #[test]
  fn every_tile_of_a_level_gets_a_distinct_contiguous_id() {
    let z = 4u8;
    let n = 1u32 << z;
    let mut tiles: Vec<(u64, u32, u32)> = (0..n)
      .flat_map(|x| (0..n).map(move |y| (x, y)))
      .map(|(x, y)| (zxy_to_tile_id(z, x, y).unwrap(), x, y))
      .collect();
    tiles.sort();
    let level_start = (4u64.pow(u32::from(z)) - 1) / 3;
    assert_eq!(tiles[0].0, level_start);
    // contiguous ids, and a curve never jumps: consecutive ids are neighbouring tiles
    for pair in tiles.windows(2) {
      assert_eq!(pair[1].0, pair[0].0 + 1);
      let dx = (i64::from(pair[1].1) - i64::from(pair[0].1)).abs();
      let dy = (i64::from(pair[1].2) - i64::from(pair[0].2)).abs();
      assert_eq!(dx + dy, 1, "{:?} -> {:?}", pair[0], pair[1]);
    }
  }

  #[test]
  fn directory_round_trip() {
    let entries = vec![
      Entry { tile_id: 0, offset: 0, length: 10, run_length: 1 },
      Entry { tile_id: 1, offset: 10, length: 20, run_length: 3 },
      Entry { tile_id: 9, offset: 500, length: 7, run_length: 1 },
      Entry { tile_id: 30, offset: 1000, length: 99, run_length: 0 },
    ];
    assert_eq!(parse_directory(&write_directory(&entries)).unwrap(), entries);
  }

  #[test]
  fn truncated_directory_is_an_error_not_a_panic() {
    let bytes = write_directory(&[Entry { tile_id: 5, offset: 0, length: 4, run_length: 1 }]);
    assert!(parse_directory(&bytes[..bytes.len() - 1]).is_err());
    assert!(parse_directory(&[0xff; 12]).is_err());
    assert!(parse_directory(&[]).is_err());
  }

  /// Builds a whole archive: a root directory with one tile run and one leaf directory.
  pub(crate) fn build_archive() -> Vec<u8> {
    let tile_a = b"tile-a".to_vec();
    let tile_b = b"tile-b".to_vec();
    let tile_deep = b"tile-deep".to_vec();
    let mut data: Vec<u8> = Vec::new();
    data.extend(&tile_a);
    data.extend(&tile_b);
    data.extend(&tile_deep);

    let leaf = gzip(&write_directory(&[Entry {
      tile_id: zxy_to_tile_id(3, 5, 6).unwrap(),
      offset: (tile_a.len() + tile_b.len()) as u64,
      length: tile_deep.len() as u32,
      run_length: 1,
    }]));
    let root = gzip(&write_directory(&[
      // z0 and z1/0/0 share tile_a through a run
      Entry { tile_id: 0, offset: 0, length: tile_a.len() as u32, run_length: 2 },
      Entry { tile_id: 2, offset: tile_a.len() as u64, length: tile_b.len() as u32, run_length: 1 },
      // everything from here on lives in the leaf
      Entry { tile_id: zxy_to_tile_id(3, 0, 0).unwrap(), offset: 0, length: leaf.len() as u32, run_length: 0 },
    ]));
    let metadata = gzip(br#"{"name":"test","minzoom":"7","maxzoom":"9","bounds":"1,2,3,4","vector_layers":[{"id":"water"}]}"#);

    let root_offset = HEADER_LEN as u64;
    let metadata_offset = root_offset + root.len() as u64;
    let leaf_offset = metadata_offset + metadata.len() as u64;
    let data_offset = leaf_offset + leaf.len() as u64;

    let mut header = vec![0u8; HEADER_LEN];
    header[0..7].copy_from_slice(MAGIC);
    header[7] = 3;
    let mut put = |pos: usize, value: u64| header[pos..pos + 8].copy_from_slice(&value.to_le_bytes());
    put(8, root_offset);
    put(16, root.len() as u64);
    put(24, metadata_offset);
    put(32, metadata.len() as u64);
    put(40, leaf_offset);
    put(48, leaf.len() as u64);
    put(56, data_offset);
    put(64, data.len() as u64);
    header[97] = 2; // internal: gzip
    header[98] = 1; // tiles: none
    header[99] = 2; // png
    header[100] = 0;
    header[101] = 3;
    header[102..106].copy_from_slice(&(-1_800_000_000i32).to_le_bytes());
    header[106..110].copy_from_slice(&(-850_000_000i32).to_le_bytes());
    header[110..114].copy_from_slice(&1_800_000_000i32.to_le_bytes());
    header[114..118].copy_from_slice(&850_000_000i32.to_le_bytes());

    let mut file = header;
    file.extend(root);
    file.extend(metadata);
    file.extend(leaf);
    file.extend(data);
    file
  }

  #[test]
  fn reads_tiles_through_runs_and_leaf_directories() {
    let dir = std::env::temp_dir().join(format!("mbview-pmtiles-test-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("test.pmtiles");
    std::fs::write(&path, build_archive()).unwrap();

    let archive = Pmtiles::open(&path).unwrap();
    assert_eq!(archive.header.tile_type, TileType::Png);
    assert_eq!(archive.header.max_zoom, 3);
    assert_eq!(archive.header.bounds, [-180.0, -85.0, 180.0, 85.0]);
    assert_eq!(archive.metadata["name"], "test");

    // the run covers tile ids 0 and 1
    assert_eq!(archive.get_tile(0, 0, 0).unwrap().unwrap(), b"tile-a");
    assert_eq!(archive.get_tile(1, 0, 0).unwrap().unwrap(), b"tile-a");
    assert_eq!(archive.get_tile(1, 0, 1).unwrap().unwrap(), b"tile-b");
    // behind a leaf directory
    assert_eq!(archive.get_tile(3, 5, 6).unwrap().unwrap(), b"tile-deep");
    // twice, to go through the leaf cache
    assert_eq!(archive.get_tile(3, 5, 6).unwrap().unwrap(), b"tile-deep");
    // absent tiles are not errors
    assert!(archive.get_tile(1, 1, 1).unwrap().is_none());
    assert!(archive.get_tile(3, 4, 4).unwrap().is_none());
    assert!(archive.get_tile(2, 9, 9).unwrap().is_none());

    std::fs::remove_dir_all(&dir).ok();
  }

  #[test]
  fn rejects_files_that_are_not_pmtiles() {
    let dir = std::env::temp_dir().join(format!("mbview-pmtiles-bad-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("bad.pmtiles");
    std::fs::write(&path, vec![0u8; 400]).unwrap();
    assert!(Pmtiles::open(&path).is_err());
    std::fs::write(&path, b"PMTiles").unwrap();
    assert!(Pmtiles::open(&path).is_err());
    std::fs::remove_dir_all(&dir).ok();
  }
}
