#![cfg_attr(
  all(not(debug_assertions), target_os = "windows"),
  windows_subsystem = "windows"
)]

#[macro_use]
extern crate lazy_static;
extern crate hyper;
extern crate md5;
extern crate r2d2;
extern crate r2d2_sqlite;
extern crate regex;

mod errors;
mod server;
mod service;
mod tiles;
mod utils;
use std::thread;

use std::env;

use std::path::PathBuf;
use tauri::Manager;
use tauri::Window;
use tauri::{AboutMetadata, CustomMenuItem, Menu, MenuItem, Submenu, WindowBuilder, WindowUrl};

use crate::tiles::{get_path, reload, set_mbtiles};

lazy_static! {
  static ref PORT: u16 = 9872;
  static ref SOURCE_NAME: String = String::from("main");
}

// the payload type must implement `Serialize`.
#[derive(Clone, serde::Serialize)]
struct Payload {
  message: String,
}
// the payload type must implement `Serialize`.
#[derive(Clone, serde::Serialize)]
struct MBtilesEventPayload {
  key: String,
  source_id: String,
  path: Option<String>,
  json_url: Option<String>,
}

#[tauri::command]
fn reload_mbtiles(path: String, window: Window) {
  // println!("reload_mbtiles {}", path);
  let mb_tiles_id = format!("{:x}", md5::compute(path.clone().as_bytes()));
  reload(&mb_tiles_id);
}
#[tauri::command]
fn setup_mbtiles(key: String, path: Option<String>, window: Window) {
  if path.is_none() {
    return;
  }
  let path_buf = PathBuf::from(path.clone().unwrap());
  if !path_buf.exists() {
    return;
  }
  let window_ = window.clone();
  let mb_tiles_id = format!("{:x}", md5::compute(path.clone().unwrap().as_bytes()));

  // println!("setup_mbtiles {} {}", path.clone().unwrap(), mb_tiles_id);
  set_mbtiles(
    &mb_tiles_id,
    path_buf,
    Box::new(move |mb_tiles_id| {
      window_
        .emit_all(
          "reload-mbtiles",
          Payload {
            message: mb_tiles_id,
          },
        )
        .unwrap()
    }),
  );
  let file_path = Some(
    get_path(&mb_tiles_id)
      .unwrap()
      .into_os_string()
      .into_string()
      .unwrap(),
  );
  window
    .emit(
      "mbtiles",
      MBtilesEventPayload {
        key: key.clone(),
        source_id: mb_tiles_id.clone(),
        path: file_path.clone(),
        json_url: if file_path.is_none() {
          None
        } else {
          Some(format!(
            "http://localhost:{}/{}/tiles.json",
            PORT.to_string(),
            mb_tiles_id.clone()
          ))
        },
      },
    )
    .unwrap();
  // }
}

fn main() {
  let ctx = tauri::generate_context!();
  thread::spawn(move || {
    if let Err(_) = server::run(*PORT) {
      std::process::exit(1);
    }
  });
  let mut menu = Menu::new();
  #[cfg(target_os = "macos")]
  {
    menu = menu.add_submenu(Submenu::new(
      &ctx.package_info().name,
      Menu::with_items([
        MenuItem::About(ctx.package_info().name.clone(), AboutMetadata::new()).into(),
        MenuItem::Separator.into(),
        MenuItem::Services.into(),
        MenuItem::Separator.into(),
        MenuItem::Hide.into(),
        MenuItem::HideOthers.into(),
        MenuItem::ShowAll.into(),
        MenuItem::Separator.into(),
        MenuItem::Quit.into(),
      ]),
    ))
  }
  
  menu = menu.add_submenu(Submenu::new(
    "File",
    Menu::with_items([
      CustomMenuItem::new("open", "Open...")
        .accelerator("CmdOrControl+O")
        .into(),
      MenuItem::CloseWindow.into(),
    ]),
  )).add_submenu(Submenu::new(
    "Edit",
    Menu::with_items([
      MenuItem::Separator.into(),
      MenuItem::Copy.into(),
      #[cfg(not(target_os = "macos"))]
      MenuItem::Separator.into(),
    ]),
  )).add_submenu(Submenu::new(
    "View",
    Menu::with_items([MenuItem::EnterFullScreen.into()]),
  )).add_submenu(Submenu::new(
    "Window",
    Menu::with_items([MenuItem::Minimize.into(), MenuItem::Zoom.into()]),
  )).add_submenu(Submenu::new(
    "Help",
    Menu::with_items([CustomMenuItem::new("learn_more", "Learn More").into()]),
  ));
  tauri::Builder::default()
    .invoke_handler(tauri::generate_handler![setup_mbtiles, reload_mbtiles])
    .plugin(tauri_plugin_window_state::Builder::default().build())
    .menu(menu)
    .setup(|app| {
      WindowBuilder::new(app, "main", WindowUrl::default())
      .title("MBTiles Viewer")
      // .resizable(true)
        // .decorations(true)
        // .always_on_top(false)
        // .inner_size(1400.0, 850.0)
        .inner_size(1400.0, 850.0)
        .min_inner_size(400.0, 200.0)
        // .skip_taskbar(false)
        // .fullscreen(false)
        .build()?;
      Ok(())
    })
    .run(ctx)
    .expect("error while running tauri application");
}
