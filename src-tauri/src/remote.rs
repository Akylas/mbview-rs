//! Plain HTTP access for sources that do not live on disk: PMTiles archives read with range
//! requests, TileJSON documents, and `{z}/{x}/{y}` tile templates.
//!
//! Everything here is blocking (`ureq`), so callers on the async server go through
//! `spawn_blocking`.

use std::io::Read;
use std::time::Duration;

use crate::errors::{Error, Result};

/// A TileJSON larger than this is not a TileJSON.
pub const MAX_JSON: u64 = 16 * 1024 * 1024;
/// Upper bound for one tile.
pub const MAX_TILE: u64 = 64 * 1024 * 1024;

lazy_static! {
  static ref AGENT: ureq::Agent = ureq::AgentBuilder::new()
    .timeout_connect(Duration::from_secs(10))
    .timeout_read(Duration::from_secs(20))
    .user_agent("MBTilesViewer")
    .build();
}

/// Whether a "path" the user gave us is really a web address.
pub fn is_remote(path: &str) -> bool {
  let lower = path.trim_start().to_ascii_lowercase();
  lower.starts_with("http://") || lower.starts_with("https://")
}

/// The URL without its query string or fragment, which is what its extension is judged on.
pub fn path_part(url: &str) -> &str {
  url.split(|c| c == '?' || c == '#').next().unwrap_or(url)
}

pub struct Reply {
  pub status: u16,
  pub body: Vec<u8>,
}

fn send(url: &str, range: Option<(u64, u64)>, limit: u64) -> Result<Reply> {
  let mut request = AGENT.get(url);
  if let Some((offset, length)) = range {
    request = request.set("Range", &format!("bytes={}-{}", offset, offset + length - 1));
  }
  let response = match request.call() {
    Ok(response) => response,
    // an error status is an answer ("no such tile"), not a failure to reach the server
    Err(ureq::Error::Status(_, response)) => response,
    Err(err) => return Err(Error::Remote(format!("{}: {}", url, err))),
  };
  let status = response.status();
  let mut body = Vec::new();
  if status < 400 && status != 204 {
    response
      .into_reader()
      .take(limit)
      .read_to_end(&mut body)
      .map_err(|err| Error::Remote(format!("{}: {}", url, err)))?;
  }
  Ok(Reply { status, body })
}

/// A whole document or tile. `Ok(None)` when the server says it has nothing there.
pub fn get(url: &str, limit: u64) -> Result<Option<Vec<u8>>> {
  let reply = send(url, None, limit)?;
  match reply.status {
    200..=299 if reply.status != 204 => Ok(Some(reply.body)),
    204 | 404 | 410 => Ok(None),
    status => Err(Error::Remote(format!("{}: HTTP {}", url, status))),
  }
}

/// Exactly `length` bytes at `offset`. A server that answers a range request with the whole
/// resource (a plain 200) is refused: its bytes would be the wrong ones.
pub fn get_range(url: &str, offset: u64, length: u64) -> Result<Vec<u8>> {
  if length == 0 {
    return Ok(Vec::new());
  }
  let reply = send(url, Some((offset, length)), length)?;
  match reply.status {
    206 if reply.body.len() as u64 == length => Ok(reply.body),
    206 => Err(Error::Remote(format!(
      "{}: asked for {} bytes at {}, got {}",
      url,
      length,
      offset,
      reply.body.len()
    ))),
    200 => Err(Error::Remote(format!(
      "{}: the server does not support range requests",
      url
    ))),
    status => Err(Error::Remote(format!("{}: HTTP {}", url, status))),
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn tells_web_addresses_from_paths() {
    assert!(is_remote("https://tiles.example.com/planet"));
    assert!(is_remote("HTTP://example.com/a.pmtiles"));
    assert!(!is_remote("/Users/me/a.pmtiles"));
    assert!(!is_remote("C:\\tiles\\a.mbtiles"));
    assert!(!is_remote("httpfoo"));
  }

  #[test]
  fn extension_ignores_the_query() {
    assert_eq!(path_part("https://a.b/c.pmtiles?token=x#y"), "https://a.b/c.pmtiles");
    assert_eq!(path_part("https://a.b/c"), "https://a.b/c");
  }
}
