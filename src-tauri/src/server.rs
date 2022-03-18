use hyper::service::{make_service_fn, service_fn};
use hyper::Server;
// use notify::Watcher;

use crate::service;

#[tokio::main]
pub async fn run(port: u16) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
  let addr = ([0, 0, 0, 0], port).into();
  let server = Server::try_bind(&addr)?;
  //   let tilesets = create_tilesets(path.clone());

  let service = {
    make_service_fn(move |_conn| {
      // let allowed_hosts = args.allowed_hosts.clone();
      // let headers = args.headers.clone();
      // let disable_preview = args.disable_preview;
      // let allow_reload_api = args.allow_reload_api;
      async move {
        Ok::<_, hyper::Error>(service_fn(move |req| {
          service::get_service(req)
        }))
      }
    })
  };

  //   move || {
  //     let wtilesets = tilesets.clone();
  //       println!("Reloading on SIGHUP");
  //     tokio::spawn(async move {
  //       let mut handler =
  //         tokio::signal::unix::signal(tokio::signal::unix::SignalKind::hangup()).unwrap();
  //       loop {
  //         handler.recv().await;
  //         println!("Caught SIGHUP, reloading tilesets");
  //         wtilesets.reload();
  //       }
  //     });
  //   }

  //   if let Some(interval) = args.reload_interval {
  //     let tilesets = args.tilesets.clone();
  //     println!("Reloading every {} seconds", interval.as_secs());
  //     tokio::spawn(async move {
  //       let mut timer = tokio::time::interval(interval);
  //       loop {
  //         timer.tick().await;
  //         tilesets.reload();
  //       }
  //     });
  //   }

  // if !args.disable_watcher {
  //   println!("Watching FS for changes on {:?}", tilesets.get_path());
  //   let watcherTileset = tilesets.clone();
  //   drop(std::thread::spawn(move || {
  //     let (tx, rx) = std::sync::mpsc::channel();
  //     let mut watcher = notify::watcher(tx, Duration::from_secs(10)).unwrap();
  //     watcher
  //       .watch(&watcherTileset.get_path(), notify::RecursiveMode::Recursive)
  //       .unwrap();
  //     loop {
  //       match rx.recv() {
  //         Ok(_) => {
  //           println!("Tileset changed, reloading");
  //           watcherTileset.reload()
  //         }
  //         Err(e) => println!("watch error: {:?}", e),
  //       }
  //     }
  //   }));
  // }

  println!("Listening on http://{}", addr);
  server.serve(service).await?;

  Ok(())
}
