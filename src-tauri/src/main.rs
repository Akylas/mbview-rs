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
use std::thread;

use std::env;

use std::path::PathBuf;
use tauri::Manager;
use tauri::Window;
use tauri::{CustomMenuItem, Menu, MenuEntry, MenuItem, Submenu, WindowBuilder, WindowUrl};

use crate::tiles::{get_path, set_mbtiles};

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
  path: Option<String>,
  json_url: Option<String>,
}

#[tauri::command]
fn setup_mbtiles(key: String, path: Option<String>, window: Window) {
  if path.is_none() {
    return;
  }
  let window_ = window.clone();
  set_mbtiles(
    &key,
    PathBuf::from(path.unwrap().clone()),
    Box::new(move |key| {
      window_
        .emit_all("reload-mbtiles", Payload { message: key })
        .unwrap()
    }),
  );
  let file_path = Some(
    get_path(&key)
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
        path: file_path.clone(),
        json_url: if file_path.is_none() {
          None
        } else {
          Some(format!(
            "http://localhost:{}/{}/tiles.json",
            PORT.to_string(),
            key
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

  tauri::Builder::default()
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
        Menu::with_items([CustomMenuItem::new("open", "Open...").accelerator("CmdOrControl+O").into(), MenuItem::CloseWindow.into()]),
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
        Menu::with_items([CustomMenuItem::new("learn_more", "Learn More").into()]),
      )),
    ]))
    .run(ctx)
    .expect("error while running tauri application");
}
