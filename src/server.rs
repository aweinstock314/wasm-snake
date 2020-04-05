#![cfg(feature="server-deps")]

use futures::future;
use futures_util::{FutureExt, StreamExt};
use futures_util::sink::SinkExt;
use tokio::sync::mpsc;
use warp::Filter;
use warp::ws::{Ws, Message};

pub mod common;

macro_rules! load_asset {
    ($name:literal) => {{
        #[cfg(feature="server-statically-pack-assets")] {
            &include_bytes!(concat!("../", $name))[..]
        }
        #[cfg(not(feature="server-statically-pack-assets"))] {
            use ::std::io::Read;
            let mut data = vec![];
            let mut file = ::std::fs::File::open($name).unwrap();
            file.read_to_end(&mut data).unwrap();
            data
        }
    }}
}

#[tokio::main]
async fn main() {
    let index = warp::path::end()
        .map(|| load_asset!("static/index.html"))
        .with(warp::reply::with::header("Content-type", "text/html"));

    let wasm_snake_js = warp::path!("pkg" / "wasm_snake.js")
        .map(|| load_asset!("static/pkg/wasm_snake.js"))
        .with(warp::reply::with::header("Content-type", "text/javascript"));

    let wasm_snake_wasm = warp::path!("pkg" / "wasm_snake_bg.wasm")
        .map(|| load_asset!("static/pkg/wasm_snake_bg.wasm"))
        .with(warp::reply::with::header("Content-type", "application/wasm"));

    let ws_endpoint = warp::path("client_connection")
        .and(warp::ws())
        .map(|ws: Ws| ws.on_upgrade(|websocket| {
            let (ws_tx, wx_rx) = websocket.split();
            let (tx, rx) = mpsc::unbounded_channel();
            tokio::task::spawn(rx.forward(ws_tx).map(|x| if let Err(e) = x { eprintln!("{:?}", e) }));
            let _ = tx.send(Ok(Message::text("Hello from a websocket!")));
            future::lazy(|_| ())
        }));

    let server = index
        .or(wasm_snake_js)
        .or(wasm_snake_wasm)
        .or(ws_endpoint);
    let into_ip = ([0, 0, 0, 0], 8000);
    println!("Serving on {:?}", into_ip);
    warp::serve(server).run(into_ip).await;
}
