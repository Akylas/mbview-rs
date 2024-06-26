use bytes::Bytes;
use hyper::header::{CONTENT_TYPE, HOST};
use hyper::{Request, Response, StatusCode};
use regex::Regex;
use serde_json::json;
use http_body_util::Full;
use crate::errors::Result;
use crate::tiles::{get_data, get_tile_data};
use crate::utils::{decode, get_blank_image, get_data_format, DataFormat};

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

fn not_found() -> Response<ReplyBody> {
  Response::builder()
    .status(StatusCode::NOT_FOUND)
    .body(Full::new(NOT_FOUND.into()))
    .unwrap()
}

fn no_content() -> Response<ReplyBody> {
  Response::builder()
    .status(StatusCode::NO_CONTENT)
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
  // println!("get_service {}", path);

  match TILE_URL_RE.captures(path) {
    Some(matches) => {
      let tile_path = matches.name("tile_path").unwrap().as_str().to_string();
      let tile_meta = get_data(&tile_path).unwrap();
      let z = matches.name("z").unwrap().as_str().parse::<u32>().unwrap();
      let x = matches.name("x").unwrap().as_str().parse::<u32>().unwrap();
      let y = matches.name("y").unwrap().as_str().parse::<u32>().unwrap();
      let y: u32 = (1 << z) - 1 - y;
      let data_format = matches.name("format").unwrap().as_str();
      // For future use
      let _query_string = match matches.name("query") {
        Some(q) => q.as_str(),
        None => "",
      };

      let response = Response::builder();

      return match data_format {
        "pbf" => match get_tile_data(&tile_meta.connection_pool.get().unwrap(), z, x, y) {
          Ok(data) => {
            let data_in_format = get_data_format(&data);
            return Ok(
              response
                .header(CONTENT_TYPE, DataFormat::Pbf.content_type())
                // .header("Cache-Control", "no-cache")
                .header("Access-Control-Allow-Origin", "*")
                .body(From::from(decode(data, data_in_format).unwrap()))
                .unwrap(),
            );
          }
          Err(_) => Ok(no_content()),
        },
        _ => {
          let data = match get_tile_data(&tile_meta.connection_pool.get().unwrap(), z, x, y) {
            Ok(data) => data,
            Err(_) => get_blank_image(),
          };
          Ok(
            response
              .header(CONTENT_TYPE, DataFormat::new(data_format).content_type())
              // .header("Cache-Control", "no-cache")
              .header("Access-Control-Allow-Origin", "*")
              .body(From::from(Bytes::from(data)))
              .unwrap(),
          )
        }
      };
    }
    None => {
      return match META_URL_RE.captures(path) {
        Some(matches) => {
          let host = get_host(&request).unwrap();
          let tile_path = matches.name("tile_path").unwrap().as_str();
          let tile_meta = get_data(&tile_path.to_string()).unwrap();
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
          if let Some(json_data) = &tile_meta.json {
            for (k, v) in json_data.as_object().unwrap() {
              tile_meta_json[k] = v.clone();
            }
          }

          return Ok(
            Response::builder()
              .header(CONTENT_TYPE, "application/json")
              .header("Access-Control-Allow-Origin", "*")
              .body(Full::from(serde_json::to_string(&tile_meta_json).unwrap()))
              .unwrap(),
          ); // TODO handle error
             // } else if path == "/reload" {
             // if allow_reload_api {
             // tilesets.reload();
             // return Ok(no_content());
             // } else {
             // return Ok(forbidden());
             // }
             // }
        }
        None => Ok(not_found()),
      };
    }
  };
}
