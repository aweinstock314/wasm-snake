#![cfg(feature="client-deps")]

#[macro_use] extern crate serde_derive;

use js_sys::{ArrayBuffer, Uint8Array};
use std::collections::{BTreeMap, VecDeque};
use std::sync::mpsc;
use web_sys::{Blob, Event, FileReader, KeyEvent, KeyboardEvent, MessageEvent};
use web_sys::{CanvasRenderingContext2d, HtmlCanvasElement};
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;

pub mod common;
use common::*;

fn log(msg: &str) {
    web_sys::console::log_1(&JsValue::from_str(msg));
}

fn regular_polygon_path<F: Fn(f64, f64) -> (f64, f64)>(canvas_ctx: &CanvasRenderingContext2d, n: usize, c_x: f64, c_y: f64, r: &F, start_angle: f64) {
    canvas_ctx.begin_path();
    for i in 0..n {
        let angle = (TAU * i as f64 / n as f64) + start_angle;
        let r = r(start_angle, angle);
        let x = c_x + r.0 * angle.cos();
        let y = c_y + r.1 * angle.sin();
        if i == 0 {
            canvas_ctx.move_to(x, y);
        } else {
            canvas_ctx.line_to(x, y);
        }
    }
    canvas_ctx.close_path();
}

fn snake_segment_path(canvas_ctx: &CanvasRenderingContext2d, x: f64, y: f64, w: f64, h: f64, prev_dir: Option<Direction>, current_dir: Direction, next_dir: Option<Direction>, forward_stroke: bool) {
    //let head_radius = |theta0: f64, theta: f64| if theta0.cos()*theta.cos() + theta0.sin()*theta.sin() > 0.0 { (w / 2.0,  h / 2.0) } else { (w/3.0, h/3.0) };
    let r = Vec2::new(w/2.0, h/2.0);
    let r2 = Vec2::new(w/3.0, h/3.0);
    let c = Vec2::new(x, y) + r;
    let theta = Vec2::from_angle(current_dir.radians());
    let phi = Vec2::from_angle(current_dir.radians() + TAU/4.0);
    match (prev_dir, next_dir) {
        (Some(prev_dir), Some(next_dir)) => {
            let (prev_theta, next_theta) = (prev_dir.radians(), next_dir.radians());
            let (prev_phi, next_phi) = (prev_theta + TAU / 4.0, next_theta + TAU / 4.0);
            let center_theta = 0.5 * (Vec2::from_angle(prev_theta) + Vec2::from_angle(next_theta));
            let center_phi = Vec2::new(-center_theta.y, center_theta.x);
            let mid_prev = c - r * Vec2::from_angle(prev_theta);
            let mid_next = c + r * Vec2::from_angle(next_theta);
            //let p0a = mid_prev - r2 * Vec2::from_angle(prev_phi);
            let p0b = mid_prev + r2 * Vec2::from_angle(prev_phi);
            let r_tweak = if prev_theta.cos()*next_theta.cos() + prev_theta.sin() * next_theta.sin() < 0.5 { 3f64.sqrt() } else { 1.0 };
            let ca = c - r_tweak * r2 * center_phi;
            let cb = c + r_tweak * r2 * center_phi;
            let p2a = mid_next - r2 * Vec2::from_angle(next_phi);
            //let p2b = mid_next + r2 * Vec2::from_angle(next_phi);
            if forward_stroke {
                //canvas_ctx.move_to(p0a.x, p0a.y);
                canvas_ctx.quadratic_curve_to(ca.x, ca.y, p2a.x, p2a.y);
            } else {
                //canvas_ctx.line_to(p2b.x, p2b.y);
                canvas_ctx.quadratic_curve_to(cb.x, cb.y, p0b.x, p0b.y);
                //canvas_ctx.line_to(p0a.x, p0a.y);
            }
        },
        (_, None) => {
            if forward_stroke {
                let p0 = c + r * theta;
                let mouth_a = p0 - r2 * phi;
                let mouth_b = p0 + r2 * phi;
                let side_a = c - 1.1 * r * phi;
                let side_b = c + 1.1 * r * phi;
                let p1 = c - r * theta - r2 * phi;
                let p2 = c - r * theta + r2 * phi;
                let p3 = c;
                if prev_dir.is_none() {
                    canvas_ctx.move_to(p1.x, p1.y);
                }
                canvas_ctx.quadratic_curve_to(side_a.x, side_a.y, mouth_a.x, mouth_a.y);
                canvas_ctx.line_to(p3.x, p3.y);
                canvas_ctx.line_to(mouth_b.x, mouth_b.y);
                canvas_ctx.quadratic_curve_to(side_b.x, side_b.y, p2.x, p2.y);
            }
        },
        (None, Some(_)) => {
            let p0 = c + r * theta;
            let p1 = p0 - r2 * phi;
            //let p2 = p0 + r2 * phi;
            let p3 = c - r * theta;
            if forward_stroke {
                canvas_ctx.move_to(p1.x, p1.y);
            } else {
                //canvas_ctx.move_to(p2.x, p2.y);
                canvas_ctx.quadratic_curve_to(p3.x, p3.y, p1.x, p1.y);
            }
        },
    }
}

fn pid_to_color(pid: PlayerId) -> &'static str {
    // TODO: more than 8 unique colors, but still distinguishable
    match pid.0 % 8 {
        0 => "#ff0000",
        1 => "#00ff00",
        2 => "#ffff00",
        3 => "#0000ff",
        4 => "#ff00ff",
        5 => "#00ffff",
        6 => "#ffffff",
        _ => "#000000",
    }
}

fn render_tile(canvas_ctx: &CanvasRenderingContext2d, x: f64, y: f64, w: f64, h: f64, board: &Board, coord: Coord) {
    use Tile::*;
    // TODO: cache colors
    match board[coord] {
        Empty => {
            canvas_ctx.set_fill_style(&JsValue::from_str(&"#f0f0f0"));
            canvas_ctx.fill_rect(x, y, w, h);
        },
        Wall => {
            canvas_ctx.set_fill_style(&JsValue::from_str(&"#101010"));
            canvas_ctx.fill_rect(x, y, w, h);
        },
        WormSegment { .. } => {
            canvas_ctx.set_fill_style(&JsValue::from_str(&"#f0f0f0"));
            canvas_ctx.fill_rect(x, y, w, h);
        },
        Food => {
            canvas_ctx.set_fill_style(&JsValue::from_str(&"#f0f0f0"));
            canvas_ctx.fill_rect(x, y, w, h);
            canvas_ctx.set_fill_style(&JsValue::from_str(&"#808000"));
            canvas_ctx.begin_path();
            canvas_ctx.ellipse(x+w/2.0, y+h/2.0, w/4.0, h/4.0, 0.0, 0.0, TAU).unwrap();
            canvas_ctx.fill();
        },
    }
}

fn render_board(canvas: &HtmlCanvasElement, canvas_ctx: &CanvasRenderingContext2d, board: &Board, player_segments: &BTreeMap<PlayerId, VecDeque<Coord>>) {
    let xscale = canvas.width() as f64 / board.width as f64;
    let yscale = canvas.height() as f64 / board.height as f64;

    for y in 0..board.height {
        for x in 0..board.width {
            render_tile(canvas_ctx, (x as f64)*xscale, (y as f64)*yscale, xscale, yscale, board, coord(x, y));
        }
    }
    let extract_dir = |coord: &Coord| if let Tile::WormSegment { dir, .. } = board[*coord] { Some(dir) } else { None };
    for (pid, segments) in player_segments.iter() {
        canvas_ctx.begin_path();
        let mut points = Vec::new();
        for i in 0..segments.len() {
            let prev_dir = segments.get(i-1).and_then(extract_dir);
            let dir = extract_dir(&segments[i]).unwrap();
            let next_dir = if i < segments.len() - 1 { Some(dir) } else { None };
            let point = segments[i].to_vec2() * Vec2::new(xscale, yscale);
            points.push((prev_dir, dir, next_dir, point));
            snake_segment_path(canvas_ctx, point.x, point.y, xscale, yscale, prev_dir, dir, next_dir, true);
        }
        points.reverse();
        for (prev_dir, dir, next_dir, point) in points.iter() {
            snake_segment_path(canvas_ctx, point.x, point.y, xscale, yscale, *prev_dir, *dir, *next_dir, false);
        }
        canvas_ctx.close_path();
        canvas_ctx.set_fill_style(&JsValue::from_str(pid_to_color(*pid)));
        canvas_ctx.fill();
        canvas_ctx.set_stroke_style(&JsValue::from_str(&"#101010"));
        canvas_ctx.stroke();
    }
}

fn keyevent_to_playerinput(e: &KeyboardEvent) -> Option<SnakePlayerInput> {
    use SnakePlayerInput::*;
    use Direction::*;
    match e.key_code() {
        KeyEvent::DOM_VK_UP | KeyEvent::DOM_VK_W => Some(ChangeDirection(Up)),
        KeyEvent::DOM_VK_DOWN | KeyEvent::DOM_VK_S => Some(ChangeDirection(Down)),
        KeyEvent::DOM_VK_LEFT | KeyEvent::DOM_VK_A => Some(ChangeDirection(Left)),
        KeyEvent::DOM_VK_RIGHT | KeyEvent::DOM_VK_D => Some(ChangeDirection(Right)),
        _ => None,
    }
}

enum PlayerInputDelta {
    Started(SnakePlayerInput),
    Ended(SnakePlayerInput),
}

#[wasm_bindgen]
pub fn wasm_main() -> Result<JsValue, JsValue> {
    #[global_allocator]
    static ALLOC: wee_alloc::WeeAlloc = wee_alloc::WeeAlloc::INIT;

    log("Hello to the console!");

    let window = web_sys::window().unwrap();
    let document = window.document().unwrap();
    let pre = document.get_element_by_id("logging_pre").unwrap();
    pre.set_text_content(Some("Hello, world!"));

    let (s2c_tx, s2c_rx) = mpsc::channel();

    let onmessage_closure = Closure::wrap(Box::new(move |msg: MessageEvent| {
        log(&format!("{:?}", msg.data()));
        if let Some(blob) = msg.data().dyn_ref::<Blob>() {
            // Blob.arrayBuffer is too new for the firefox that debian stable ships with
            /*let onmessage_buffer_closure = Closure::wrap(Box::new(move |buffer: JsValue| {
                let buffer: ArrayBuffer = buffer.dyn_into().unwrap();
            }) as Box<dyn FnMut(JsValue)>);
            blob.array_buffer().then(&onmessage_buffer_closure);*/
            let filereader = FileReader::new().unwrap();
            let s2c_tx = s2c_tx.clone();
            let onmessage_reader_closure = Closure::wrap(Box::new(move |e: Event| {
                let reader: FileReader = e.target().unwrap().dyn_into().unwrap();
                let buffer: ArrayBuffer = reader.result().unwrap().dyn_into().unwrap();
                let bytes = Uint8Array::new(&buffer);
                let msg = bincode::deserialize::<ServerToClient>(&bytes.to_vec());
                log(&format!("{:?}", msg));
                if let Ok(msg) = msg {
                    let _ = s2c_tx.send(msg);
                }
            }) as Box<dyn FnMut(Event)>);
            filereader.set_onload(onmessage_reader_closure.as_ref().dyn_ref());
            onmessage_reader_closure.forget();
            filereader.read_as_array_buffer(blob).unwrap();
        }
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


    // TODO: populate from websocket
    let mut our_pid = PlayerId(0);
    let mut gamestate = SnakeGameState::new();

    let mut current_inputs: BTreeMap<PlayerId, <SnakeGameState as GameState>::PlayerInput> = BTreeMap::new();
    let mut last_ts = None;
    let (input_tx, input_rx) = mpsc::channel();
    let raf_closure = Closure::wrap(Box::new(move |ts: f64| {
        if let Some(ts2) = last_ts.as_mut() {
            let seconds_since_last = (ts - *ts2)/1000.0;
            let num_ticks = (seconds_since_last*TICKS_PER_SECOND) as usize;
            while let Ok(msg) = s2c_rx.try_recv() {
                use ServerToClient::*;
                match msg {
                    Initialize { pid, world } => { our_pid = pid; gamestate = world; },
                    DoTick { tick, inputs } => { gamestate.tick(&inputs); },
                    PlayerDisconnected { pid } => { gamestate.remove_player(pid, 0); },
                }
            }
            while let Ok(input) = input_rx.try_recv() {
                match input {
                    PlayerInputDelta::Started(input) => { current_inputs.insert(our_pid, input); },
                    PlayerInputDelta::Ended(input) => if current_inputs[&our_pid] == input { current_inputs.remove(&our_pid); },
                }
            }
            if let Some(input) = current_inputs.get(&our_pid) {
                ws.send_with_u8_array(&bincode::serialize(&ClientToServer::InputAtTick { tick: gamestate.tick, input: *input }).unwrap()).unwrap();
            }
            if num_ticks > 0 {
                log(&format!("{:?} {:?}", seconds_since_last, num_ticks));
                log(&format!("current_inputs: {:?}", current_inputs));
                /*for _ in 0..num_ticks {
                    let events = gamestate.tick(&current_inputs);
                    log(&format!("events: {:?}", events));
                }*/
                *ts2 = ts;
            }
        } else {
            last_ts = Some(ts);
        }
        render_board(&canvas, &canvas_ctx, &gamestate.board, &gamestate.player_segments);
    }) as Box<dyn FnMut(f64)>);
    let raf_closure_jsval = raf_closure.as_ref().clone();
    raf_closure.forget();

    let input_tx_ = input_tx.clone();
    let keyup_closure = Closure::wrap(Box::new(move |e: KeyboardEvent| {
        log(&format!("keyup {:?}", e));
        /*if let Some(x) = keyevent_to_playerinput(&e) {
            input_tx_.send(PlayerInputDelta::Ended(x)).unwrap();
        }*/
    }) as Box<dyn FnMut(_)>);
    window.add_event_listener_with_callback("keyup", keyup_closure.as_ref().dyn_ref().unwrap()).unwrap();
    keyup_closure.forget();

    let keydown_closure = Closure::wrap(Box::new(move |e: KeyboardEvent| {
        log(&format!("keydown {:?}", e));
        if let Some(x) = keyevent_to_playerinput(&e) {
            input_tx.send(PlayerInputDelta::Started(x)).unwrap();
        }
    }) as Box<dyn FnMut(_)>);
    window.add_event_listener_with_callback("keydown", keydown_closure.as_ref().dyn_ref().unwrap()).unwrap();
    keydown_closure.forget();

    Ok(raf_closure_jsval)
}
