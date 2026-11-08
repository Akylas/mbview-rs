use bytes::Bytes;
use hyper::header::{CONTENT_TYPE, HOST};
use hyper::{Request, Response, StatusCode};
use regex::Regex;
use serde_json::json;
use http_body_util::Full;
use crate::errors::Result;
use crate::tiles::{get_data, inflate_vector};
use crate::utils::DataFormat;

type ReplyBody = Full<Bytes>;
lazy_static! {
  static ref TILE_URL_RE: Regex = Regex::new(
    r"^/(?P<tile_path>.*)/tiles/(?P<z>\d+)/(?P<x>\d+)/(?P<y>\d+)\.(?P<format>[a-zA-Z]+)"
  )
  .unwrap();
  static ref META_URL_RE: Regex = Regex::new(r"^/(?P<tile_path>.*)/tiles.json").unwrap();
}

#[allow(dead_code)]
static INTERNAL_SERVER_ERROR: &[u8] = b"Internal Server Error";
static NOT_FOUND: &[u8] = b"Not Found";
static NO_CONTENT: &[u8] = b"";

// Every answer carries the CORS header, empty ones included: the webview is another origin than
// this server, and a tile that is simply absent must read as "no tile", not as a blocked request.
fn not_found() -> Response<ReplyBody> {
  Response::builder()
    .status(StatusCode::NOT_FOUND)
    .header("Access-Control-Allow-Origin", "*")
    .body(Full::new(NOT_FOUND.into()))
    .unwrap()
}

fn no_content() -> Response<ReplyBody> {
  Response::builder()
    .status(StatusCode::NO_CONTENT)
    .header("Access-Control-Allow-Origin", "*")
    .body(Full::new(NO_CONTENT.into()))
    .unwrap()
}

#[allow(dead_code)]
fn server_error() -> Response<ReplyBody> {
  Response::builder()
    .status(StatusCode::INTERNAL_SERVER_ERROR)
    .body(Full::new(INTERNAL_SERVER_ERROR.into()))
    .unwrap()
}

fn get_host(req: &Request<hyper::body::Incoming>) -> Option<&str> {
  let host = req.uri().host();
  if host.is_some() {
    return host;
  }

  if let Some(host) = req.headers().get(HOST) {
    return Some(host.to_str().unwrap());
  }

  None
}

pub async fn get_service(request: Request<hyper::body::Incoming>) -> Result<Response<ReplyBody>> {
  let uri = request.uri();
  let path = uri.path();

  if let Some(matches) = TILE_URL_RE.captures(path) {
    let tile_path = matches.name("tile_path").unwrap().as_str().to_string();
    // anything that does not parse is a request for a tile that cannot exist
    let (z, x, y) = match (
      matches.name("z").unwrap().as_str().parse::<u32>(),
      matches.name("x").unwrap().as_str().parse::<u32>(),
      matches.name("y").unwrap().as_str().parse::<u32>(),
    ) {
      (Ok(z), Ok(x), Ok(y)) => (z, x, y),
      _ => return Ok(not_found()),
    };
    let tile_meta = match get_data(&tile_path) {
      Some(tile_meta) => tile_meta,
      None => return Ok(not_found()),
    };
    let data_format = matches.name("format").unwrap().as_str();
    let response = Response::builder();

    // `Err` is a file that went bad under us (truncated, rebuilt): serve it as an empty tile
    // rather than taking the connection, or the app, down with it.
    let stored = tile_meta.get_tile(z, x, y).unwrap_or_else(|err| {
      println!("tile {}/{}/{} of {}: {}", z, x, y, tile_meta.id, err);
      None
    });

    return match data_format {
      // vector tiles are stored gzipped, and are served uncompressed as we set no Content-Encoding
      "pbf" | "mlt" => match stored.map(inflate_vector) {
        Some(Ok(data)) => Ok(
          response
            .header(CONTENT_TYPE, DataFormat::new(data_format).content_type())
            .header("Access-Control-Allow-Origin", "*")
            .body(From::from(data))
            .unwrap(),
        ),
        Some(Err(err)) => {
          println!("tile {}/{}/{} of {}: {}", z, x, y, tile_meta.id, err);
          Ok(no_content())
        }
        None => Ok(no_content()),
      },
      _ => {
        // A 404, not a blank image: maplibre then falls back to a parent tile, which is right for
        // rasters, and for elevation a fake "empty" tile would decode as a pit thousands of metres deep.
        let data = match stored.map(inflate_vector) {
          // image tiles are sometimes stored compressed too, and a browser cannot decode that
          Some(Ok(data)) => data,
          _ => return Ok(not_found()),
        };
        Ok(
          response
            .header(CONTENT_TYPE, DataFormat::new(data_format).content_type())
            .header("Access-Control-Allow-Origin", "*")
            .body(From::from(Bytes::from(data)))
            .unwrap(),
        )
      }
    };
  }

  match META_URL_RE.captures(path) {
    Some(matches) => {
      let host = match get_host(&request) {
        Some(host) => host,
        None => return Ok(not_found()),
      };
      let tile_path = matches.name("tile_path").unwrap().as_str();
      let tile_meta = match get_data(&tile_path.to_string()) {
        Some(tile_meta) => tile_meta,
        None => return Ok(not_found()),
      };
      let mut tile_meta_json = json!({
          "name":  if tile_meta.name.is_some() { tile_meta.name.clone() } else { Some(tile_meta.id.clone()) } ,
          "version": tile_meta.version,
          "tiles": vec![format!(
              "http://{}/{}/tiles/{{z}}/{{x}}/{{y}}.{}",
              host,
              tile_path,
              tile_meta.tile_format.format(),
          )],
          "tilejson": tile_meta.tilejson,
          "scheme": tile_meta.scheme,
          "id": tile_path,
          "format": tile_meta.tile_format,
          "bounds": tile_meta.bounds,
          "center": tile_meta.center,
          "minzoom": tile_meta.minzoom,
          "maxzoom": tile_meta.maxzoom,
          "description": tile_meta.description,
          "attribution": tile_meta.attribution,
          "type": tile_meta.layer_type,
          "legend": tile_meta.legend,
          "template": tile_meta.template,
      });
      // MapLibre GL JS merges the TileJSON into the source, so this is what switches a vector
      // source over to the MLT decoder. Values come from the style spec `encoding` enum.
      if tile_meta.tile_format.is_vector() {
        tile_meta_json["encoding"] = json!(if tile_meta.tile_format == DataFormat::Mlt {
          "mlt"
        } else {
          "mvt"
        });
      }
      if let Some(json_data) = tile_meta.json.as_ref().and_then(|json| json.as_object()) {
        for (k, v) in json_data {
          // what the file says about its own addressing must not override what this server does
          // PMTiles metadata is free-form JSON and often spells numbers as strings ("maxzoom":
          // "13"); the numeric values above come from the header and must not be replaced by it.
          if matches!(
            k.as_str(),
            "tiles" | "scheme" | "tilejson" | "id" | "format" | "bounds" | "center" | "minzoom" | "maxzoom"
          ) {
            continue;
          }
          tile_meta_json[k] = v.clone();
        }
      }

      Ok(
        Response::builder()
          .header(CONTENT_TYPE, "application/json")
          .header("Access-Control-Allow-Origin", "*")
          .body(Full::from(serde_json::to_string(&tile_meta_json).unwrap_or_default()))
          .unwrap(),
      )
    }
    None => Ok(not_found()),
  }
}

#[cfg(test)]
mod tests {
  use crate::tiles::set_mbtiles;
  use std::io::{Read, Write};
  use std::net::TcpStream;
  use std::path::PathBuf;
  use std::time::Duration;

  fn get(port: u16, path: &str) -> (String, Vec<u8>) {
    let mut stream = TcpStream::connect(("127.0.0.1", port)).unwrap();
    stream.set_read_timeout(Some(Duration::from_secs(5))).unwrap();
    write!(
      stream,
      "GET {} HTTP/1.1\r\nHost: localhost:{}\r\nConnection: close\r\n\r\n",
      path, port
    )
    .unwrap();
    let mut raw = Vec::new();
    stream.read_to_end(&mut raw).unwrap();
    let split = raw.windows(4).position(|w| w == b"\r\n\r\n").unwrap();
    (
      String::from_utf8_lossy(&raw[..split]).into_owned(),
      raw[split + 4..].to_vec(),
    )
  }

  /// The whole path a webview takes: open, ask for tiles.json, ask for tiles. Bad requests must
  /// answer with a status and leave the server alive, which a panic in a handler would not.
  #[test]
  fn serves_a_pmtiles_archive_over_http() {
    let dir = std::env::temp_dir().join(format!("mbview-http-test-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let path: PathBuf = dir.join("served.pmtiles");
    std::fs::write(&path, crate::pmtiles::tests::build_archive()).unwrap();

    let port = 19873;
    std::thread::spawn(move || {
      let _ = crate::server::run(port);
    });
    std::thread::sleep(Duration::from_millis(300));

    let key = String::from("served");
    set_mbtiles(&key, path, Box::new(|_| ())).unwrap();

    let (head, body) = get(port, "/served/tiles.json");
    assert!(head.starts_with("HTTP/1.1 200"), "{}", head);
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json["format"], "png");
    assert_eq!(json["scheme"], "xyz");
    // the archive's own metadata spells these as strings; the header's numbers must win
    assert_eq!(json["maxzoom"], 3);
    assert_eq!(json["minzoom"], 0);
    assert_eq!(json["bounds"][0], -180.0);
    assert_eq!(json["tiles"][0], "http://localhost:19873/served/tiles/{z}/{x}/{y}.png");
    assert_eq!(json["vector_layers"][0]["id"], "water");

    let (head, body) = get(port, "/served/tiles/1/0/1.png");
    assert!(head.starts_with("HTTP/1.1 200"), "{}", head);
    assert!(head.to_lowercase().contains("content-type: image/png"));
    assert_eq!(body, b"tile-b");

    // an absent image tile is a plain 404 (maplibre treats that as "no data here")
    let (head, _) = get(port, "/served/tiles/1/1/1.png");
    assert!(head.starts_with("HTTP/1.1 404"), "{}", head);

    // unknown file, junk numbers: a status line each, and the server keeps answering
    let (head, _) = get(port, "/nope/tiles.json");
    assert!(head.starts_with("HTTP/1.1 404"), "{}", head);
    assert!(head.to_lowercase().contains("access-control-allow-origin: *"), "{}", head);
    let (head, _) = get(port, "/nope/tiles/1/0/0.pbf");
    assert!(head.starts_with("HTTP/1.1 404"), "{}", head);
    let (head, _) = get(port, "/served/tiles/99999999999/0/0.png");
    assert!(head.starts_with("HTTP/1.1 404"), "{}", head);
    let (head, _) = get(port, "/served/tiles.json");
    assert!(head.starts_with("HTTP/1.1 200"), "{}", head);

    std::fs::remove_dir_all(&dir).ok();
  }

  /// Serves the fixtures named in MBVIEW_SERVE (comma separated paths) until killed, for looking
  /// at them in a browser: `MBVIEW_SERVE=a.pmtiles cargo test serve_fixtures -- --ignored`.
  #[test]
  #[ignore]
  fn serve_fixtures() {
    let list = match std::env::var("MBVIEW_SERVE") {
      Ok(list) => list,
      Err(_) => return,
    };
    std::thread::spawn(|| {
      let _ = crate::server::run(9872);
    });
    for file in list.split(',') {
      let id = format!("{:x}", md5::compute(file.as_bytes()));
      set_mbtiles(&id, PathBuf::from(file), Box::new(|_| ())).unwrap();
      println!("{} -> http://localhost:9872/{}/tiles.json", file, id);
    }
    std::thread::sleep(Duration::from_secs(3600));
  }
}
