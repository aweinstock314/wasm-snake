#![cfg(feature="server-deps")]

use warp::Filter;

#[tokio::main]
async fn main() {
    let index = warp::path::end()
        .map(|| &include_bytes!("../static/index.html")[..])
        .with(warp::reply::with::header("Content-type", "text/html"));

    let wasm_snake_js = warp::path!("pkg" / "wasm_snake.js")
        .map(|| &include_bytes!("../static/pkg/wasm_snake.js")[..])
        .with(warp::reply::with::header("Content-type", "text/javascript"));

    let wasm_snake_wasm = warp::path!("pkg" / "wasm_snake_bg.wasm")
        .map(|| &include_bytes!("../static/pkg/wasm_snake_bg.wasm")[..])
        .with(warp::reply::with::header("Content-type", "application/wasm"));

    let server = index.or(wasm_snake_js).or(wasm_snake_wasm);
    println!("Serving on 127.0.0.1:8000");
    warp::serve(server).run(([127, 0, 0, 1], 8000)).await;
}
