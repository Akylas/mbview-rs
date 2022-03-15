#![cfg_attr(
  all(not(debug_assertions), target_os = "windows"),
  windows_subsystem = "windows"
)]
#[macro_use]
extern crate lazy_static;
extern crate hyper;
extern crate r2d2;
extern crate r2d2_sqlite;
extern crate regex;

mod errors;
mod server;
mod service;
mod tiles;
mod utils;

// use notify::Watcher;
// use regex::Regex;
// use serde_json::json;
use std::sync::Mutex;
use std::thread;

use std::env;

// use std::path::Path;
use std::path::PathBuf;
// use tauri::http::ResponseBuilder;
use tauri::{CustomMenuItem, Menu, MenuEntry, MenuItem, Submenu, WindowBuilder, WindowUrl};
use tauri::{State, Window};

use crate::tiles::{create_tilesets, Tilesets};
// use crate::utils::{decode, get_blank_image, get_data_format, DataFormat};

lazy_static! {
  static ref PORT: u16 = 9872;
  static ref SOURCE_NAME: String = String::from("main");
  // static ref TILE_URL_RE: Regex = Regex::new(
  //   r"^mbtiles://(?P<tile_path>.*)/tiles/(?P<z>\d+)/(?P<x>\d+)/(?P<y>\d+)\.(?P<format>[a-zA-Z]+)"
  // )
  // .unwrap();
  // static ref META_URL_RE: Regex = Regex::new(r"^mbtiles://(?P<tile_path>.*)/tiles.json").unwrap();
}

// struct MBTiles {
//   path: Option<String>,
//   metadata: Option<TileMeta>,
//   watcher: Option<notify::FsEventWatcher>,
// }

// #[allow(dead_code)]
// static NOT_FOUND: &[u8] = b"Not Found";
// static CONTENT_TYPE: &[u8] = b"Content-Type";

// the payload type must implement `Serialize`.
#[derive(serde::Serialize)]
struct Payload {
  message: Option<String>,
}
// the payload type must implement `Serialize`.
#[derive(Clone, serde::Serialize)]
struct MBtilesEventPayload {
  path: Option<String>,
  json_url: Option<String>,
}

// fn closeConnection(connection: Connection) {
//   connection.close();
// }

#[tauri::command]
fn setup_mbtiles(
  path: Option<String>,
  tilesets: State<Mutex<Tilesets>>,
  // app_handle: tauri::AppHandle,
  window: Window,
) {
  if path.is_none() {
    return;
  }
  println!("setup_mbtiles {}", path.clone().unwrap());
  let the_tilesets = tilesets.lock().unwrap();
  the_tilesets.set_path(PathBuf::from(path.unwrap().clone()));
  // if !data.metadata.is_none() {
  //   // let connection = data.metadata.take().unwrap().connection_pool.get().unwrap();
  //   data.metadata = None;
  //   data.path = None;
  // }
  // if !path.is_none() {
  //   let c_path = path.clone().unwrap();
  //   if c_path.starts_with("asset://") {
  //     data.path = Some(c_path.replace("asset://", ""));
  //     // data.path = match resolve_path(
  //     //   app_handle.config().as_ref(),
  //     //   app_handle.package_info(),
  //     //   c_path.replace("asset://", ""),
  //     //   None,
  //     // ) {
  //     //   Ok(res) => Some(res.into_os_string().into_string().unwrap()),
  //     //   Err(_) => None,
  //     // };
  //   } else {
  //     data.path = Some(path.clone().unwrap());
  //   }
  //   if !data.path.is_none() {
  //     let file_path = Path::new(data.path.as_ref().unwrap());
  //     data.metadata = match get_tile_details(&file_path) {
  //       Ok(tile_meta) => Some(tile_meta),
  //       Err(err) => None,
  //     };
  //   }
  let file_path = Some(
    the_tilesets
      .get_path()
      .into_os_string()
      .into_string()
      .unwrap(),
  );
  window
    .emit(
      "mbtiles",
      MBtilesEventPayload {
        path: file_path.clone(),
        json_url: if file_path.is_none() {
          None
        } else {
          Some(format!(
            "http://localhost:{}/{}/tiles.json",
            PORT.to_string(),
            SOURCE_NAME.clone()
          ))
        },
      },
    )
    .unwrap();
  // }
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
}

fn main() {
  let ctx = tauri::generate_context!();
  let tilesets = create_tilesets(None);
  let stilesets = tilesets.clone();

  thread::spawn(move || {
    if let Err(_) = server::run(*PORT, stilesets.clone()) {
      // error!("Server error: {}", e);
      std::process::exit(1);
    }
  });
  println!("server started");
  // let mutex = Mutex::new(tilesets);
  // let mbtiles = Mutex::new(MBTiles {
  //   path: None,
  //   metadata: None,
  //   watcher: None,
  // });
  tauri::Builder::default()
    .manage(Mutex::new(tilesets.clone()))
    .invoke_handler(tauri::generate_handler![setup_mbtiles])
    .create_window("main", WindowUrl::default(), |win, webview| {
      let win = win
        .title("MBTiles Viewer")
        .resizable(true)
        .decorations(true)
        .always_on_top(false)
        .inner_size(1400.0, 850.0)
        .min_inner_size(400.0, 200.0)
        .skip_taskbar(false)
        .fullscreen(false);
      return (win, webview);
    })
    .unwrap() // safe to unwrap: window label is valid
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
    // .on_menu_event(|event| {
    //   let event_name = event.menu_item_id();
    //   println!("on_menu_event {:?}", event_name);
    //   match event_name {
    //     "Learn More" => {
    //       open(
    //         "https://github.com/probablykasper/tauri-template".to_string(),
    //         None,
    //       )
    //       .unwrap();
    //     }
    //     _ => {}
    //   }
    // })
    // .register_uri_scheme_protocol("mbtiles", move |_app, request| {
    //   let mbtiles: State<Mutex<MBTiles>> = _app.state();
    //   let data = mbtiles.lock().unwrap();
    //   // prepare our response
    //   let response = ResponseBuilder::new();
    //   if data.metadata.is_none() {
    //     return response.status(404).body(NOT_FOUND.into());
    //   }
    //   let tile_meta = data.metadata.as_ref().unwrap();
    //   match TILE_URL_RE.captures(request.uri()) {
    //     Some(matches) => {
    //       // let tile_path = matches.name("tile_path").unwrap().as_str();
    //       let z = matches.name("z").unwrap().as_str().parse::<u32>().unwrap();
    //       let x = matches.name("x").unwrap().as_str().parse::<u32>().unwrap();
    //       let y = matches.name("y").unwrap().as_str().parse::<u32>().unwrap();
    //       let y: u32 = (1 << z) - 1 - y;
    //       let data_format = matches.name("format").unwrap().as_str();
    //       return match data_format {
    //         "pbf" => match get_tile_data(&tile_meta.connection_pool.get().unwrap(), z, x, y) {
    //           Ok(data) => {
    //             let data_in_format = get_data_format(&data);
    //             match data_in_format {
    //               DataFormat::Gzip => Ok(
    //                 response
    //                   .mimetype(DataFormat::Pbf.content_type())
    //                   .header("Access-Control-Allow-Origin", "*")
    //                   .body(decode(data, data_in_format).unwrap())
    //                   .unwrap(),
    //               ),
    //               _ => Ok(
    //                 response
    //                   .header(CONTENT_TYPE, DataFormat::Pbf.content_type())
    //                   .header("Access-Control-Allow-Origin", "*")
    //                   .body(data)
    //                   .unwrap(),
    //               ),
    //             }
    //           }
    //           Err(_) => response.status(404).body(NOT_FOUND.into()),
    //         },
    //         _ => {
    //           let data = match get_tile_data(&tile_meta.connection_pool.get().unwrap(), z, x, y) {
    //             Ok(data) => data,
    //             Err(_) => get_blank_image(),
    //           };
    //           Ok(
    //             response
    //               .header(CONTENT_TYPE, DataFormat::new(data_format).content_type())
    //               .header("Access-Control-Allow-Origin", "*")
    //               .body(data)
    //               .unwrap(),
    //           )
    //         }
    //       };
    //     }
    //     None => {
    //       return match META_URL_RE.captures(request.uri()) {
    //         Some(matches) => {
    //           let tile_path = matches.name("tile_path").unwrap().as_str();
    //           let mut tile_meta_json = json!({
    //               "name": tile_meta.name,
    //               "version": tile_meta.version,
    //               "tiles": vec![format!(
    //                   "mbtiles://{}/tiles/{{z}}/{{x}}/{{y}}.{}",
    //                   tile_path,
    //                   tile_meta.tile_format.format(),
    //               )],
    //               "tilejson": tile_meta.tilejson,
    //               "scheme": tile_meta.scheme,
    //               "id": tile_meta.id,
    //               "format": tile_meta.tile_format,
    //               "bounds": tile_meta.bounds,
    //               "center": tile_meta.center,
    //               "minzoom": tile_meta.minzoom,
    //               "maxzoom": tile_meta.maxzoom,
    //               "description": tile_meta.description,
    //               "attribution": tile_meta.attribution,
    //               "type": tile_meta.layer_type,
    //               "legend": tile_meta.legend,
    //               "template": tile_meta.template,
    //           });
    //           if let Some(json_data) = &tile_meta.json {
    //             for (k, v) in json_data.as_object().unwrap() {
    //               tile_meta_json[k] = v.clone();
    //             }
    //           }
    //           return response
    //             .header(CONTENT_TYPE, "application/json")
    //             .header("Access-Control-Allow-Origin", "*")
    //             .body(serde_json::to_string(&tile_meta_json).unwrap().into());
    //         }
    //         None => response.status(404).body(NOT_FOUND.into()),
    //       };
    //     }
    //   };
    // })
    .run(ctx)
    .expect("error while running tauri application");
}
