use std::io::prelude::*;

use flate2::read::{GzDecoder, ZlibDecoder};
use flate2::write::GzEncoder;
use flate2::Compression;

use serde::{Deserialize, Serialize};

use crate::errors::{Error, Result};

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum DataFormat {
  Png,
  Jpg,
  Webp,
  Json,
  Pbf,
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
      "pbf" => DataFormat::Pbf,
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
      _ => "",
    }
  }

  pub fn content_encoding(&self) -> &str {
    match *self {
      DataFormat::Gzip => "gzip",
      DataFormat::Zlib => "deflate",
      DataFormat::Br => "br",
      _ => "",
    }
  }
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
        _ => Err(Error::InvalidDataFormat(String::from(data_type.format()))),
    }
}

pub fn encode(data: &[u8]) -> Vec<u8> {
  let mut e = GzEncoder::new(Vec::new(), Compression::default());
  e.write_all(data).unwrap();
  e.finish().unwrap()
}

pub fn get_data_format(data: &[u8]) -> DataFormat {
  match data {
    v if &v[0..2] == b"\x1f\x8b" => DataFormat::Gzip,
    v if &v[0..2] == b"\x78\x9c" => DataFormat::Zlib,
    v if &v[0..8] == b"\x89\x50\x4E\x47\x0D\x0A\x1A\x0A" => DataFormat::Png,
    v if &v[0..3] == b"\xFF\xD8\xFF" => DataFormat::Jpg,
    v if &v[0..4] == b"RIFF" && &v[8..12] == b"WEBP" => DataFormat::Webp,
    _ => DataFormat::Unknown,
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
}
