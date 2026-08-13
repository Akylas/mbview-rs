use std::io::prelude::*;

use flate2::read::{GzDecoder, ZlibDecoder};
// use flate2::write::GzEncoder;
// use flate2::Compression;

use serde::{Deserialize, Serialize};

use crate::errors::Result;

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum DataFormat {
  Png,
  Jpg,
  Webp,
  Json,
  Pbf,
  Mlt,
  Br,
  Gzip,
  Zlib,
  Unknown,
}

impl DataFormat {
  pub fn new(format: &str) -> DataFormat {
    match format {
      "png" => DataFormat::Png,
      "jpg" | "jpeg" => DataFormat::Jpg,
      "webp" => DataFormat::Webp,
      "json" => DataFormat::Json,
      "pbf" | "mvt" => DataFormat::Pbf,
      "mlt" => DataFormat::Mlt,
      "gzip" => DataFormat::Gzip,
      "zlib" => DataFormat::Zlib,
      "br" => DataFormat::Br,
      _ => DataFormat::Unknown,
    }
  }

  pub fn format(&self) -> &str {
    match *self {
      DataFormat::Png => "png",
      DataFormat::Jpg => "jpg",
      DataFormat::Webp => "webp",
      DataFormat::Json => "json",
      DataFormat::Pbf => "pbf",
      DataFormat::Mlt => "mlt",
      _ => "",
    }
  }

  pub fn content_type(&self) -> &str {
    match *self {
      DataFormat::Png => "image/png",
      DataFormat::Jpg => "image/jpeg",
      DataFormat::Webp => "image/webp",
      DataFormat::Json => "application/json",
      DataFormat::Pbf => "application/x-protobuf",
      DataFormat::Mlt => "application/vnd.maplibre-tile",
      _ => "",
    }
  }

  /// Formats holding vector features, i.e. the ones a `vector` source can render.
  pub fn is_vector(&self) -> bool {
    matches!(*self, DataFormat::Pbf | DataFormat::Mlt)
  }

  // pub fn content_encoding(&self) -> &str {
  //   match *self {
  //     DataFormat::Gzip => "gzip",
  //     DataFormat::Zlib => "deflate",
  //     DataFormat::Br => "br",
  //     _ => "",
  //   }
  // }
}

pub fn decode(data: Vec<u8>, data_type: DataFormat) -> Result<Vec<u8>> {
    match data_type {
        DataFormat::Gzip => {
            let mut z = GzDecoder::new(&data[..]);
            let mut s = Vec::new();
            z.read_to_end(&mut s).unwrap();
            Ok(s)
        }
        DataFormat::Zlib => {
            let mut z = ZlibDecoder::new(&data[..]);
            let mut s = Vec::new();
            z.read_to_end(&mut s).unwrap();
            Ok(s)
        }
        _ => Ok(data),
    }
}

// pub fn encode(data: &[u8]) -> Vec<u8> {
//   let mut e = GzEncoder::new(Vec::new(), Compression::default());
//   e.write_all(data).unwrap();
//   e.finish().unwrap()
// }

pub fn get_data_format(data: &[u8]) -> DataFormat {
  match data {
    v if v.starts_with(b"\x1f\x8b") => DataFormat::Gzip,
    v if v.starts_with(b"\x78\x9c") => DataFormat::Zlib,
    v if v.starts_with(b"\x89\x50\x4E\x47\x0D\x0A\x1A\x0A") => DataFormat::Png,
    v if v.starts_with(b"\xFF\xD8\xFF") => DataFormat::Jpg,
    v if v.starts_with(b"RIFF") && v.len() >= 12 && &v[8..12] == b"WEBP" => DataFormat::Webp,
    _ => DataFormat::Unknown,
  }
}

/// A MapLibre Tile has no magic bytes: it is a bare sequence of `varint(size), tag, payload`
/// blocks, where `size` covers the tag byte plus the payload but not the varint itself, and
/// `tag` identifies the block type (`0x01` = feature table v1, the only tag defined so far).
/// A buffer is taken for MLT when it parses as such a sequence all the way to its end.
///
/// MVT cannot be mistaken for MLT: it starts with the protobuf tag of its `layers` field,
/// which puts a protobuf field header where the block tag would be, and field 0 does not exist.
pub fn is_mlt(data: &[u8]) -> bool {
  if data.is_empty() {
    return false;
  }
  let mut pos = 0usize;
  while pos < data.len() {
    let (size, varint_len) = match read_varint(&data[pos..]) {
      Some(res) => res,
      None => return false,
    };
    pos += varint_len;
    let block_end = match size.checked_add(pos as u64) {
      Some(end) if end <= data.len() as u64 => end as usize,
      _ => return false, // block overruns the tile
    };
    // an empty block has no room for its own tag
    if data.get(pos) != Some(&0x01) {
      return false;
    }
    pos = block_end;
  }
  true
}

/// Reads one LEB128 varint, returning its value and the number of bytes it took.
fn read_varint(data: &[u8]) -> Option<(u64, usize)> {
  let mut value = 0u64;
  for (i, byte) in data.iter().enumerate() {
    if i >= 10 {
      return None; // longer than a u64 can hold
    }
    value |= u64::from(byte & 0x7f) << (i * 7);
    if byte & 0x80 == 0 {
      return Some((value, i + 1));
    }
  }
  None // truncated
}

/// Format of a tile blob, looking through the gzip/zlib compression tiles are usually stored with.
/// Vector payloads carry no magic bytes, so anything that is not a known image and does not parse
/// as MLT is assumed to be MVT, as that is by far the most common case.
pub fn get_tile_format(data: &[u8]) -> DataFormat {
  let stored_as = get_data_format(data);
  match stored_as {
    DataFormat::Gzip | DataFormat::Zlib => match decode(data.to_vec(), stored_as) {
      Ok(decoded) => get_vector_format(&decoded),
      Err(_) => DataFormat::Pbf,
    },
    DataFormat::Unknown => get_vector_format(data),
    known => known,
  }
}

fn get_vector_format(data: &[u8]) -> DataFormat {
  if is_mlt(data) {
    DataFormat::Mlt
  } else {
    DataFormat::Pbf
  }
}

pub fn get_blank_image() -> Vec<u8> {
  let image = b"\x89PNG\r\n\x1a\n\x00\x00\x00\rIHDR\x00\x00\x01\x00\x00\x00\x01\x00\x01\x03\x00\x00\x00f\xbc:%\x00\x00\x00\x03PLTE\x00\x00\x00\xa7z=\xda\x00\x00\x00\x01tRNS\x00@\xe6\xd8f\x00\x00\x00\x1fIDATh\xde\xed\xc1\x01\r\x00\x00\x00\xc2 \xfb\xa76\xc77`\x00\x00\x00\x00\x00\x00\x00\x00q\x07!\x00\x00\x01\xa7W)\xd7\x00\x00\x00\x00IEND\xaeB`\x82";
  image.to_vec()
}

#[cfg(test)]
mod tests {
  use super::*;
  use std::fs::read;

  #[test]
  fn test_data_format_png() {
    assert_eq!(
      get_data_format(&read("./tiles/world.png").unwrap()),
      DataFormat::Png
    );
  }

  #[test]
  fn test_data_format_jpg() {
    assert_eq!(
      get_data_format(&read("./tiles/world.jpg").unwrap()),
      DataFormat::Jpg
    );
  }

  #[test]
  fn test_data_format_webp() {
    assert_eq!(
      get_data_format(&read("./tiles/dc.webp").unwrap()),
      DataFormat::Webp
    );
  }

  #[test]
  fn test_is_mlt() {
    // single block: size = 4 (tag byte + 3 payload bytes), tag = 0x01
    assert!(is_mlt(&[0x04, 0x01, 0xaa, 0xbb, 0xcc]));
    // two consecutive blocks
    assert!(is_mlt(&[0x02, 0x01, 0xaa, 0x03, 0x01, 0xbb, 0xcc]));
    // a multi-byte (LEB128) size
    let mut big = vec![0x81, 0x01, 0x01];
    big.extend(std::iter::repeat(0xaa).take(128));
    assert!(is_mlt(&big));
    // unknown tag, truncated payload, trailing garbage and empty input are not MLT
    assert!(!is_mlt(&[0x04, 0x02, 0xaa, 0xbb, 0xcc]));
    assert!(!is_mlt(&[0x04, 0x01, 0xaa]));
    assert!(!is_mlt(&[0x02, 0x01, 0xaa, 0xff]));
    assert!(!is_mlt(&[]));
  }

  #[test]
  fn test_mvt_is_not_mlt() {
    // an MVT tile starts with field 3 (layer), length delimited, then the layer's own fields
    assert!(!is_mlt(&[0x1a, 0x0f, 0x0a, 0x05, b'w', b'a', b't', b'e', b'r']));
  }
}
