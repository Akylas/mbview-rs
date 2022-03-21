use hyper::service::{make_service_fn, service_fn};
use hyper::Server;
// use notify::Watcher;

use crate::service;

#[tokio::main]
pub async fn run(port: u16) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
  let addr = ([0, 0, 0, 0], port).into();
  let server = Server::try_bind(&addr)?;
  let service = {
    make_service_fn(move |_conn| {
      // let allowed_hosts = args.allowed_hosts.clone();
      // let headers = args.headers.clone();
      async move {
        Ok::<_, hyper::Error>(service_fn(move |req| {
          service::get_service(req)
        }))
      }
    })
  };
  server.serve(service).await?;
  Ok(())
}
