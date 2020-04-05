#![cfg(feature="client-deps")]

use web_sys::MessageEvent;
use web_sys::{CanvasRenderingContext2d, HtmlCanvasElement};
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;

pub mod common;
use common::*;

fn log(msg: &str) {
    web_sys::console::log_1(&JsValue::from_str(msg));
}

fn regular_polygon_path(canvas_ctx: &CanvasRenderingContext2d, n: usize, c_x: f64, c_y: f64, r_x: f64, r_y: f64, start_angle: f64) {
    canvas_ctx.begin_path();
    for i in 0..n {
        let angle = (TAU * i as f64 / n as f64) - start_angle;
        let x = c_x + r_x * angle.cos();
        let y = c_y + r_y * angle.sin();
        if i == 0 {
            canvas_ctx.move_to(x, y);
        } else {
            canvas_ctx.line_to(x, y);
        }
    }
    canvas_ctx.close_path();
}

fn render_tile(canvas_ctx: &CanvasRenderingContext2d, x: f64, y: f64, w: f64, h: f64, tile: Tile) {
    use Tile::*;
    // TODO: cache colors
    match tile {
        Empty => {
            canvas_ctx.set_fill_style(&JsValue::from_str(&"#f0f0f0"));
            canvas_ctx.fill_rect(x, y, w, h);
        },
        Wall => {
            canvas_ctx.set_fill_style(&JsValue::from_str(&"#101010"));
            canvas_ctx.fill_rect(x, y, w, h);
        },
        WormSegment { pid, dir } => {
            render_tile(canvas_ctx, x, y, w, h, Tile::Empty);
            // TODO: color based on pid
            canvas_ctx.set_fill_style(&JsValue::from_str(&"#ff0000"));
            let (r_x, r_y) = match dir {
                Direction::Left | Direction::Right => (w/2.0, h/4.0),
                Direction::Up | Direction::Down => (w/4.0, h/2.0),
            };
            regular_polygon_path(canvas_ctx, 3, x+(w/2.0), y+(h/2.0), r_x, r_y, dir.radians());
            canvas_ctx.fill();
        },
        Food => {
        },
    }
}

fn render_board(canvas: &HtmlCanvasElement, canvas_ctx: &CanvasRenderingContext2d, board: &Board) {
    let xscale = canvas.width() as f64 / board.width as f64;
    let yscale = canvas.height() as f64 / board.height as f64;

    for y in 0..board.height {
        for x in 0..board.width {
            render_tile(canvas_ctx, (x as f64)*xscale, (y as f64)*yscale, xscale, yscale, board[coord(x, y)]);
        }
    }
}

#[wasm_bindgen]
pub fn wasm_main() -> Result<(), JsValue> {
    #[global_allocator]
    static ALLOC: wee_alloc::WeeAlloc = wee_alloc::WeeAlloc::INIT;

    log("Hello to the console!");

    let window = web_sys::window().unwrap();
    let document = window.document().unwrap();
    let pre = document.get_element_by_id("logging_pre").unwrap();
    pre.set_text_content(Some("Hello, world!"));

    let onmessage_closure = Closure::wrap(Box::new(move |msg: MessageEvent| {
        if let Some(data) = msg.data().as_string() {
            pre.set_text_content(Some(&data));
        }
    }) as Box<dyn FnMut(MessageEvent)>);

    let host = document.location().and_then(|loc| loc.host().ok()).unwrap();
    let ws = web_sys::WebSocket::new(&format!("ws://{}/client_connection", host)).unwrap();
    ws.set_onmessage(onmessage_closure.as_ref().dyn_ref());

    onmessage_closure.forget();

    let canvas: HtmlCanvasElement = document.get_element_by_id("game_canvas").and_then(|x| x.dyn_into().ok()).unwrap();
    log(&format!("{:?}", canvas));
    let canvas_ctx: CanvasRenderingContext2d = canvas.get_context("2d").ok().flatten().and_then(|x| x.dyn_into().ok()).unwrap();
    log(&format!("{:?}", canvas_ctx));

    let mut board = Board::new(30, 30);
    board.place_player(2, 2, PlayerId(0), Direction::Right);
    board.place_player(3, 2, PlayerId(0), Direction::Down);
    board.place_player(3, 3, PlayerId(0), Direction::Left);
    board.place_player(2, 3, PlayerId(0), Direction::Down);
    render_board(&canvas, &canvas_ctx, &board);

    Ok(())
}
