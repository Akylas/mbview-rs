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
use tauri::menu::{
  MenuBuilder, MenuItemBuilder, SubmenuBuilder,
};
use tauri::Manager;
use tauri::WebviewWindow;
use tauri::{WebviewWindowBuilder, WebviewUrl};

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
  source_type: Option<String>,
  layer_type: Option<String>,
  json_url: Option<String>,
}

#[tauri::command]
fn reload_mbtiles(path: String, _window: WebviewWindow) {
  // println!("reload_mbtiles {}", path);
  let mb_tiles_id = format!("{:x}", md5::compute(path.clone().as_bytes()));
  reload(&mb_tiles_id);
}
#[tauri::command]
fn setup_mbtiles(
  key: String,
  path: Option<String>,
  source_type: Option<String>,
  layer_type: Option<String>,
  window: WebviewWindow,
) {
  if path.is_none() {
    return;
  }
  let path_buf = PathBuf::from(path.clone().unwrap());
  if !path_buf.exists() {
    return;
  }
  let window_ = window.clone();
  let mb_tiles_id = format!("{:x}", md5::compute(path.clone().unwrap().as_bytes()));

  // println!(
  //   "setup_mbtiles {} {} {} {}",
  //   path.clone().unwrap(),
  //   mb_tiles_id,
  //   source_type.as_ref().map_or("not found", String::as_str),
  //   layer_type.as_ref().map_or("not found", String::as_str)
  // );
  set_mbtiles(
    &mb_tiles_id,
    path_buf,
    Box::new(move |mb_tiles_id| {
      window_
        .emit(
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
        source_type: source_type.clone(),
        layer_type: layer_type.clone(),
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

  tauri::Builder::default()
    .invoke_handler(tauri::generate_handler![setup_mbtiles, reload_mbtiles])
    .plugin(tauri_plugin_window_state::Builder::default().build())
    .plugin(tauri_plugin_fs::init())
    .plugin(tauri_plugin_dialog::init())
    .plugin(tauri_plugin_fs::init())
    .setup(|app| {
      let  mut menu_builder = MenuBuilder::new(app);
      #[cfg(target_os = "macos")]
      {
        menu_builder = menu_builder.item(&SubmenuBuilder::new(app, &ctx.package_info().name)
        .about(Some(AboutMetadataBuilder::new().build()))
        .separator()
        .services()
        .separator()
        .hide()
        .hide_others()
        .show_all()
        .separator()
        .quit()
        .build()?);
      }
      let open = MenuItemBuilder::with_id("open", "Open...")
      .accelerator("CmdOrControl+O")
      .build(app)?;
      menu_builder = menu_builder
        .item(
          &SubmenuBuilder::new(app, "File")
            .item(&open)
            .close_window()
            .build()?,
        )
        .item(
          &SubmenuBuilder::new(app, "Help")
            .item(&MenuItemBuilder::with_id("learn_more", "Learn More").build(app)?)
            .close_window()
            .build()?,
        );
      // menu = menu
      //   .item(Submenu::new(
      //     "File",
      //     Menu::with_items([
      //       CustomMenuItem::new("open", "Open...")
      //         .accelerator("CmdOrControl+O")
      //         .into(),
      //       MenuItem::CloseWindow.into(),
      //     ]),
      //   ))
      //   // .add_submenu(Submenu::new(
      //   //   "Edit",
      //   //   Menu::with_items([
      //   //     MenuItem::Copy.into()
      //   //   ]),
      //   // ))
      //   // .add_submenu(Submenu::new(
      //   //   "View",
      //   //   Menu::with_items([MenuItem::EnterFullScreen.into()]),
      //   // ))
      //   // .add_submenu(Submenu::new(
      //   //   "Window",
      //   //   Menu::with_items([MenuItem::Minimize.into(), MenuItem::Zoom.into()]),
      //   // ))
      //   .add_submenu(Submenu::new(
      //     "Help",
      //     Menu::with_items([CustomMenuItem::new("learn_more", "Learn More").into()]),
      //   ));
      app.set_menu(menu_builder.build()?)?;
      WebviewWindowBuilder::new(app, "main", WebviewUrl::default())
        .title("MBTiles Viewer")
        // .resizable(true)
        // .decorations(true)
        // .always_on_top(false)
        // .inner_size(1400.0, 850.0)
        .inner_size(1400.0, 850.0)
        .min_inner_size(400.0, 200.0)
        // .additional_browser_args("--disable-features=msWebOOUI,msPdfOOUI,msSmartScreenProtection --disable-gpu")
        // .skip_taskbar(false)
        // .fullscreen(false)
        .build()?;
      Ok(())
    })
    .run(ctx)
    .expect("error while running tauri application");
}
