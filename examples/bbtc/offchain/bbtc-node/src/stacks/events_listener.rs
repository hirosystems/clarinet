use async_std::net::{TcpStream, TcpListener};
use async_std::prelude::*;
use async_std::task;
use http_types::{Response, StatusCode};
use std::thread;

pub fn start_events_listener() {
    thread::spawn(move || {
        task::block_on(start_server());     // For each incoming TCP connection, spawn a task and call `accept`.
    let mut incoming = listener.incoming();
    
    while let Some(stream) = incoming.next().await {
        let stream = stream?;
        task::spawn(async {
            if let Err(err) = accept(stream).await {
                eprintln!("{}", err);
            }
        });
    }
    Ok(())
}


// Take a TCP stream, and convert it into sequential HTTP request / response pairs.
async fn accept(stream: TcpStream) -> http_types::Result<()> {
    println!("starting new connection from {}", stream.peer_addr()?);
    async_h1::accept(stream.clone(), |_req| async move {
        let mut res = Response::new(StatusCode::Ok);
        res.insert_header("Content-Type", "text/plain");
        res.set_body("Hello");
        Ok(res)
    })
    .await?;
    Ok(())
}
