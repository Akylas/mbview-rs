#![cfg_attr(
  all(not(debug_assertions), target_os = "windows"),
  windows_subsystem = "windows"
)]
#[macro_use]
extern crate lazy_static;
extern crate r2d2;
extern crate r2d2_sqlite;
extern crate regex;

mod errors;
mod tiles;
mod utils;

use notify::Watcher;
use regex::Regex;
use serde_json::json;
use std::sync::Mutex;
use std::time::Duration;

use std::env;
use std::fs::File;
use std::io::Write;

use std::path::Path;
use tauri::api::{
  path::{resolve_path, BaseDirectory},
  shell,
};
use tauri::{CustomMenuItem, Menu, MenuEntry, MenuItem, Submenu, WindowBuilder, WindowUrl};
use tauri::{Manager, State, Window};

use crate::tiles::{get_tile_data, get_tile_details, TileMeta};
use crate::utils::{decode, encode, get_blank_image, get_data_format, DataFormat};

lazy_static! {
  static ref TILE_URL_RE: Regex = Regex::new(
    r"^mbtiles://(?P<tile_path>.*)/tiles/(?P<z>\d+)/(?P<x>\d+)/(?P<y>\d+)\.(?P<format>[a-zA-Z]+)"
  )
  .unwrap();
  static ref META_URL_RE: Regex = Regex::new(r"^mbtiles://(?P<tile_path>.*)/tiles.json").unwrap();
}

struct MBTiles {
  path: Option<String>,
  metadata: Option<TileMeta>,
  watcher: Option<notify::FsEventWatcher>,
}

#[allow(dead_code)]
static NOT_FOUND: &[u8] = b"Not Found";
static CONTENT_TYPE: &[u8] = b"Content-Type";
static CONTENT_ENCODING: &[u8] = b"Content-Encoding";

// the payload type must implement `Serialize`.
#[derive(serde::Serialize)]
struct Payload {
  message: String,
}

// fn closeConnection(connection: Connection) {
//   connection.close();
// }

#[tauri::command]
fn setup_mbtiles(
  path: Option<String>,
  mbtiles: State<Mutex<MBTiles>>,
  app_handle: tauri::AppHandle,
  window: Window,
) {
  let app_dir = app_handle.path_resolver().app_dir();
  let mut data = mbtiles.lock().unwrap();
  if !data.metadata.is_none() {
    let connection = data.metadata.take().unwrap().connection_pool.get().unwrap();
    data.metadata = None;
    data.path = None;
  }
  if !path.is_none() {
    let c_path = path.clone().unwrap();
    println!("c_path {}", c_path);
    if c_path.starts_with("asset://") {
      data.path = Some(c_path.replace("asset://", ""));
      // data.path = match resolve_path(
      //   app_handle.config().as_ref(),
      //   app_handle.package_info(),
      //   c_path.replace("asset://", ""),
      //   None,
      // ) {
      //   Ok(res) => Some(res.into_os_string().into_string().unwrap()),
      //   Err(_) => None,
      // };
    } else {
      data.path = Some(path.clone().unwrap());
    }
    println!("data.ath {}", data.path.as_ref().unwrap());
    if !data.path.is_none() {
      let file_path = Path::new(data.path.as_ref().unwrap());
      data.metadata = match get_tile_details(&file_path) {
        Ok(tile_meta) => Some(tile_meta),
        Err(err) => None,
      };
    }
  }
  //   drop(std::thread::spawn(move || {
  //     let (tx, rx) = std::sync::mpsc::channel();
  //     let mut watcher = notify::watcher(tx, Duration::from_secs(10)).unwrap();
  //     data.watcher = Some(watcher);
  //     watcher
  //         .watch(&file_path, notify::RecursiveMode::NonRecursive)
  //         .unwrap();
  //     loop {
  //         match rx.recv() {
  //             Ok(_) => {
  //                 println!("MBTiles changed, reloading");
  //             }
  //             Err(e) => println!("watch error: {:?}", e),
  //         }
  //     }
  // }));
  println!("data.metadata {}", data.path.as_ref().unwrap());
  window
    .emit(
      "mbtiles",
      Payload {
        message: data.path.as_ref().unwrap().into(),
      },
    )
    .unwrap();
}

fn main() {
  let ctx = tauri::generate_context!();
  use tauri::http::ResponseBuilder;
  let mbtiles = Mutex::new(MBTiles {
    path: None,
    metadata: None,
    watcher: None,
  });
  tauri::Builder::default()
    .manage(mbtiles)
    .invoke_handler(tauri::generate_handler![setup_mbtiles])
    .create_window("main", WindowUrl::default(), |win, webview| {
      let win = win
        .title("MBTiles Viewer")
        .resizable(true)
        .transparent(false)
        .decorations(true)
        .always_on_top(false)
        .inner_size(1000.0, 850.0)
        .min_inner_size(400.0, 200.0)
        .skip_taskbar(false)
        .fullscreen(false);

      // if let Err(e) = server::run(win) {
      //     error!("Server error: {}", e);
      //     std::process::exit(1);
      // }
      return (win, webview);
    })
    .menu(Menu::with_items([
      #[cfg(target_os = "macos")]
      MenuEntry::Submenu(Submenu::new(
        &ctx.package_info().name,
        Menu::with_items([
          MenuItem::About(ctx.package_info().name.clone()).into(),
          MenuItem::Separator.into(),
          MenuItem::Services.into(),
          MenuItem::Separator.into(),
          MenuItem::Hide.into(),
          MenuItem::HideOthers.into(),
          MenuItem::ShowAll.into(),
          MenuItem::Separator.into(),
          MenuItem::Quit.into(),
        ]),
      )),
      MenuEntry::Submenu(Submenu::new(
        "File",
        Menu::with_items([MenuItem::CloseWindow.into()]),
      )),
      MenuEntry::Submenu(Submenu::new(
        "Edit",
        Menu::with_items([
          MenuItem::Undo.into(),
          MenuItem::Redo.into(),
          MenuItem::Separator.into(),
          MenuItem::Cut.into(),
          MenuItem::Copy.into(),
          MenuItem::Paste.into(),
          #[cfg(not(target_os = "macos"))]
          MenuItem::Separator.into(),
          MenuItem::SelectAll.into(),
        ]),
      )),
      MenuEntry::Submenu(Submenu::new(
        "View",
        Menu::with_items([MenuItem::EnterFullScreen.into()]),
      )),
      MenuEntry::Submenu(Submenu::new(
        "Window",
        Menu::with_items([MenuItem::Minimize.into(), MenuItem::Zoom.into()]),
      )),
      // You should always have a Help menu on macOS because it will automatically
      // show a menu search field
      MenuEntry::Submenu(Submenu::new(
        "Help",
        Menu::with_items([CustomMenuItem::new("Learn More", "Learn More").into()]),
      )),
    ]))
    .on_menu_event(|event| {
      let event_name = event.menu_item_id();
      println!("on_menu_event {:?}", event_name);
      match event_name {
        "Learn More" => {
          shell::open(
            "https://github.com/probablykasper/tauri-template".to_string(),
            None,
          )
          .unwrap();
        }
        _ => {}
      }
    })
    .register_uri_scheme_protocol("mbtiles", move |_app, request| {
      let mbtiles: State<Mutex<MBTiles>> = _app.state();
      let data = mbtiles.lock().unwrap();
      // prepare our response
      let response = ResponseBuilder::new();
      if data.metadata.is_none() {
        return response.status(404).body(NOT_FOUND.into());
      }
      let tile_meta = data.metadata.as_ref().unwrap();

      println!("request.uri() {}", request.uri());

      match TILE_URL_RE.captures(request.uri()) {
        Some(matches) => {
          // let tile_path = matches.name("tile_path").unwrap().as_str();
          let z = matches.name("z").unwrap().as_str().parse::<u32>().unwrap();
          let x = matches.name("x").unwrap().as_str().parse::<u32>().unwrap();
          let y = matches.name("y").unwrap().as_str().parse::<u32>().unwrap();
          let y: u32 = (1 << z) - 1 - y;
          let data_format = matches.name("format").unwrap().as_str();
          return match data_format {
            "pbf" => match get_tile_data(&tile_meta.connection_pool.get().unwrap(), z, x, y) {
              Ok(data) => {
                let data_in_format = get_data_format(&data);

                // // Create a temporary file.
                // let temp_directory = env::temp_dir();
                // let temp_file = temp_directory.join(format!("tile.{}.{}.{}.pbf", z, x, y));
                // let temp_file_path = String::from(temp_file.clone().as_path().to_string_lossy());

                // // Open a file in write-only (ignoring errors).
                // // This creates the file if it does not exist (and empty the file if it exists).
                // let mut file = File::create(temp_file).unwrap();

                // // Write a &str in the file (ignoring the result).
                // file.write(&data);
                println!(
                  "pbf {} {}",
                  request.uri(),
                  data_in_format.content_encoding(),
                  // temp_file_path
                );
                match data_in_format {
                  DataFormat::Gzip => Ok(
                    response
                      .header(CONTENT_TYPE, DataFormat::Pbf.content_type())
                      .header(CONTENT_ENCODING, data_in_format.content_encoding())
                      .status(200)
                      .header("Access-Control-Allow-Origin", "*")
                      .body(data)
                      .unwrap(),
                  ),
                  _ => Ok(
                    response
                      .header(CONTENT_TYPE, DataFormat::Pbf.content_type())
                      .header("Access-Control-Allow-Origin", "*")
                      .header(CONTENT_ENCODING, DataFormat::Gzip.format())
                      .body(encode(&data))
                      .unwrap(),
                  ),
                }
              }
              Err(_) => response.status(404).body(NOT_FOUND.into()),
            },
            _ => {
              let data = match get_tile_data(&tile_meta.connection_pool.get().unwrap(), z, x, y) {
                Ok(data) => data,
                Err(_) => get_blank_image(),
              };
              Ok(
                response
                  .header(CONTENT_TYPE, DataFormat::new(data_format).content_type())
                  .header("Access-Control-Allow-Origin", "*")
                  .body(data)
                  .unwrap(),
              )
            }
          };
        }
        None => {
          return match META_URL_RE.captures(request.uri()) {
            Some(matches) => {
              let tile_path = matches.name("tile_path").unwrap().as_str();
              let mut tile_meta_json = json!({
                  "name": tile_meta.name,
                  "version": tile_meta.version,
                  "tiles": vec![format!(
                      "mbtiles://{}/tiles/{{z}}/{{x}}/{{y}}.{}",
                      tile_path,
                      tile_meta.tile_format.format(),
                      // query_string
                  )],
                  "tilejson": tile_meta.tilejson,
                  "scheme": tile_meta.scheme,
                  "id": tile_meta.id,
                  "format": tile_meta.tile_format,
                  // "grids": tile_meta.grid_format.map(|_| vec![format!(
                  //     "{}/{}/tiles/{{z}}/{{x}}/{{y}}.json{}",
                  //     base_url, tile_name, query_string
                  // )]),
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
              return response
                .header(CONTENT_TYPE, "application/json")
                .header("Access-Control-Allow-Origin", "*")
                .body(serde_json::to_string(&tile_meta_json).unwrap().into());
            }
            None => response.status(404).body(NOT_FOUND.into()),
          };
        }
      };
    })
    .run(ctx)
    .expect("error while running tauri application");
}
