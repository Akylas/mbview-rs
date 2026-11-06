use std::error::Error as StdError;
use std::fmt;
use std::io::Error as IoError;
use std::result::Result as StdResult;

use r2d2::Error as R2D2Error;
use rusqlite::Error as DBError;

pub type Result<T> = StdResult<T, Error>;

#[derive(Debug)]
pub enum Error {
  DBConnection(DBError),
  Pool(R2D2Error),
  Io(IoError),
  // Config(String),
  MissingTable(String),
  InvalidDataFormat(String),
  InvalidDataFormatQueryCategory(String),
  UnknownTileFormat(String),
  /// The file claims to be a PMTiles archive but cannot be read as one.
  InvalidPmtiles(String),
  /// The file is neither an MBTiles (SQLite) database nor a PMTiles archive.
  UnsupportedFile(String),
}

impl fmt::Display for Error {
  fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
    match self {
      Error::DBConnection(err) => write!(f, "Database error: {}", err),
      Error::Pool(err) => write!(f, "Connection pool error: {}", err),
      Error::Io(err) => write!(f, "I/O error: {}", err),
      Error::MissingTable(tile_name) => {
        write!(f, "Missing tiles or metadata tables: {}", tile_name)
      }
      Error::InvalidDataFormat(data_format) => {
        write!(f, "Invalid data format: {}", data_format)
      }
      Error::InvalidDataFormatQueryCategory(tile_name) => {
        write!(f, "Invalid query category: {}", tile_name)
      }
      Error::UnknownTileFormat(tile_name) => write!(f, "Unknown tile format: {}", tile_name),
      Error::InvalidPmtiles(reason) => write!(f, "Invalid PMTiles archive: {}", reason),
      Error::UnsupportedFile(tile_name) => {
        write!(f, "Not an MBTiles or PMTiles file: {}", tile_name)
      }
    }
  }
}

impl StdError for Error {}

impl From<IoError> for Error {
  fn from(err: IoError) -> Self {
    Error::Io(err)
  }
}
