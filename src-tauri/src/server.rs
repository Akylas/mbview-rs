use crate::service;
use hyper::server::conn::http1;
use hyper::service::service_fn;
use hyper_util::rt::tokio::TokioIo;
use std::net::SocketAddr;
use tokio::net::TcpListener;
// #[tokio::main]
// pub async fn run(port: u16) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
//   let addr = ([0, 0, 0, 0], port).into();
//   let server = Server::try_bind(&addr)?;
//   let service = {
//     service_fn(move |_conn| {
//       // let allowed_hosts = args.allowed_hosts.clone();
//       // let headers = args.headers.clone();
//       async move {
//         Ok::<_, hyper::Error>(service_fn(move |req| {
//           service::get_service(req)
//         }))
//       }
//     })
//   };
//   server.serve(service).await?;
//   Ok(())
// }
#[tokio::main]
pub async fn run(port: u16) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
  let addr = SocketAddr::from(([0, 0, 0, 0], port));

  // We create a TcpListener and bind it to 127.0.0.1:3000
  let listener = TcpListener::bind(addr).await?;

  // We start a loop to continuously accept incoming connections
  loop {
    let (stream, _) = listener.accept().await?;

    // Use an adapter to access something implementing `tokio::io` traits as if they implement
    // `hyper::rt` IO traits.
    let io = TokioIo::new(stream);

    // Spawn a tokio task to serve multiple connections concurrently
    tokio::task::spawn(async move {
      let service = service_fn(move |req| service::get_service(req));

      if let Err(err) = http1::Builder::new().serve_connection(io, service).await {
        println!("Failed to serve connection: {:?}", err);
      }
      // Finally, we bind the incoming connection to our `hello` service
      // if let Err(err) = http1::Builder::new()
      //     // `service_fn` converts our function in a `Service`
      //     .serve_connection(io, service_fn(move |req| service::get_service(req)))
      //     .await
      // {
      //     println!("Error serving connection: {:?}", err);
      // }
    });
  }
}
