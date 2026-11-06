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
mod pmtiles;
mod server;
mod service;
mod tiles;
mod utils;
use std::thread;

use std::path::PathBuf;
use tauri::menu::{AboutMetadataBuilder, MenuBuilder, MenuItemBuilder, SubmenuBuilder};
use tauri::Emitter;
use tauri::WebviewWindow;
use tauri::{WebviewUrl, WebviewWindowBuilder};

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
async fn reload_mbtiles(path: String, _window: WebviewWindow) -> Result<(), String> {
  let mb_tiles_id = format!("{:x}", md5::compute(path.as_bytes()));
  // reopening a big archive is disk work: keep it off the thread that drives the UI
  tauri::async_runtime::spawn_blocking(move || reload(&mb_tiles_id))
    .await
    .map_err(|err| err.to_string())
}

/// Opens a tile file and announces it to the webview. Async and blocking-pool backed on purpose:
/// a synchronous command runs on the main thread, and opening a large file there freezes the
/// whole window until it returns.
#[tauri::command]
async fn setup_mbtiles(
  key: String,
  path: Option<String>,
  source_type: Option<String>,
  layer_type: Option<String>,
  window: WebviewWindow,
) -> Result<(), String> {
  let path = match path {
    Some(path) => path,
    None => return Ok(()),
  };
  let path_buf = PathBuf::from(&path);
  if !path_buf.exists() {
    return Err(format!("File not found: {}", path));
  }
  let mb_tiles_id = format!("{:x}", md5::compute(path.as_bytes()));

  let window_ = window.clone();
  let id = mb_tiles_id.clone();
  let opened = tauri::async_runtime::spawn_blocking(move || {
    set_mbtiles(
      &id,
      path_buf,
      Box::new(move |mb_tiles_id| {
        let _ = window_.emit(
          "reload-mbtiles",
          Payload {
            message: mb_tiles_id,
          },
        );
      }),
    )
  })
  .await
  .map_err(|err| err.to_string())?;
  opened.map_err(|err| err.to_string())?;

  let file_path = get_path(&mb_tiles_id).and_then(|path| path.into_os_string().into_string().ok());
  window
    .emit(
      "mbtiles",
      MBtilesEventPayload {
        key,
        source_id: mb_tiles_id.clone(),
        json_url: file_path.as_ref().map(|_| {
          format!(
            "http://localhost:{}/{}/tiles.json",
            PORT.to_string(),
            mb_tiles_id
          )
        }),
        path: file_path,
        source_type,
        layer_type,
      },
    )
    .map_err(|err| err.to_string())
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
    .plugin(tauri_plugin_clipboard_manager::init())
    .plugin(tauri_plugin_opener::init())
    .plugin(tauri_plugin_dialog::init())
    .plugin(tauri_plugin_fs::init())
    .on_menu_event(|app, event| {
      // Forward menu clicks to the webview; the frontend owns the actual behaviour.
      let _ = app.emit(
        "menu",
        Payload {
          message: event.id().0.clone(),
        },
      );
    })
    .setup(move |app| {
      let mut menu_builder = MenuBuilder::new(app);
      #[cfg(target_os = "macos")]
      {
        menu_builder = menu_builder.item(
              &SubmenuBuilder::new(app, &app.package_info().name)
                .about(Some(AboutMetadataBuilder::new().build()))
                .separator()
                .services()
                .separator()
                .hide()
                .hide_others()
                .show_all()
                .separator()
                .quit()
                .build()?,
            );
      }
      menu_builder = menu_builder
        .item(
          &SubmenuBuilder::new(app, "File")
            .item(
              &MenuItemBuilder::with_id("open", "Open...")
                .accelerator("CmdOrControl+O")
                .build(app)?,
            )
            .close_window()
            .build()?,
        )
        .item(
          &SubmenuBuilder::new(app, "Help")
            .item(&MenuItemBuilder::with_id("learn_more", "Learn More").build(app)?)
            .close_window()
            .build()?,
        );
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
