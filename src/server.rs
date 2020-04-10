#![cfg(feature="server-deps")]

#[macro_use] extern crate serde_derive;

use futures::{future, Future};
use futures_util::{FutureExt, StreamExt, TryStreamExt};
use futures_util::sink::SinkExt;
use std::collections::HashMap;
use tokio::sync::mpsc::{self, UnboundedSender, UnboundedReceiver};
use tokio::time::{Duration, interval};
use warp::Filter;
use warp::ws::{Ws, WebSocket, Message};

pub mod common;
use common::*;

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

    let (server_tx, server_rx) = mpsc::unbounded_channel();
    let server_tx_ = server_tx.clone();
    let ws_endpoint = warp::path("client_connection")
        .and(warp::ws())
        .map(move |ws: Ws| { let tmp = server_tx_.clone(); ws.on_upgrade(move |websocket| handle_client_connection(tmp.clone(), websocket)) });

    tokio::task::spawn({
        let mut server_state = ServerGameState::new();
        server_rx.for_each(move |msg| server_state.handle_msg(msg))
    });

    let server_tx_ = server_tx.clone();
    tokio::task::spawn(interval(Duration::from_millis(250)).for_each(move |_| { let _ = server_tx_.send(ServerInternalMsg::DoTick); future::ready(()) }));

    let state_endpoint = warp::path("state")
        .and_then({
            async fn tmp(server_tx: UnboundedSender<ServerInternalMsg>) -> Result<String, warp::Rejection> {
                let (tx, mut rx) = mpsc::unbounded_channel();
                Ok(match server_tx.send(ServerInternalMsg::GetCurrentState(tx)) {
                    Ok(()) => match rx.recv().await {
                        Some(state) => state,
                        None => format!("recv() failed"),
                    }
                    Err(e) => format!("send() failed: {:?}", e),
                })
            }
            move || { let server_tx_ = server_tx.clone(); tmp(server_tx_.clone()) }
        })
        .with(warp::reply::with::header("Content-type", "text/plain"));

    let server = index
        .or(wasm_snake_js)
        .or(wasm_snake_wasm)
        .or(ws_endpoint)
        .or(state_endpoint);

    let into_ip = ([0, 0, 0, 0], 8000);
    println!("Serving on {:?}", into_ip);
    warp::serve(server).run(into_ip).await;
}

#[derive(Debug)]
enum ServerInternalMsg {
    PlayerConnected(UnboundedSender<ServerToClient>, UnboundedReceiver<ClientToServer>),
    GetCurrentState(UnboundedSender<String>),
    DoTick,
}

#[derive(Debug)]
struct ServerGameState {
    next_pid: PlayerId,
    game_state: GameState,
    channels: HashMap<PlayerId, (UnboundedSender<ServerToClient>, UnboundedReceiver<ClientToServer>)>,
    player_inputs: HashMap<PlayerId, PlayerInput>,
}

impl ServerGameState {
    fn new() -> ServerGameState {
        ServerGameState {
            next_pid: PlayerId(0),
            game_state: GameState::new(),
            channels: HashMap::new(),
            player_inputs: HashMap::new(),
        }
    }
    fn handle_msg(&mut self, msg: ServerInternalMsg) -> impl Future<Output=()> {
        use ServerInternalMsg::*;
        match msg {
            PlayerConnected(tx, rx) => {
                let pid = self.next_pid;
                self.next_pid.0 += 1;
                println!("ServerGameState::handle_msg: PlayerConnected {:?}", pid);
                self.game_state.spawn_player(pid);
                let _ = tx.send(ServerToClient::Initialize { pid, world: self.game_state.clone() });
                for (pid, (tx, _)) in self.channels.iter_mut() {
                    // TODO: lighter-weight way of notifying of new players
                    let _ = tx.send(ServerToClient::Initialize { pid: *pid, world: self.game_state.clone() });
                }
                self.channels.insert(pid, (tx, rx));
                future::ready(())
            }
            GetCurrentState(tx) => {
                let _ = tx.send(format!("{:?}", self));
                future::ready(())
            }
            DoTick => {
                for (pid, (_, rx)) in self.channels.iter_mut() {
                    while let Ok(c2s) = rx.try_recv() {
                        use ClientToServer::*;
                        match c2s {
                            InputAtTick { tick, input } => {
                                // TODO: rollback and replay world or discard input based on how recent it is, and send a sparser response
                                self.player_inputs.insert(*pid, input);
                                //let _ = tx.send(ServerToClient::Initialize { pid: *pid, world: self.game_state.clone() });
                            },
                        }
                    }
                }
                for (pid, (tx, _)) in self.channels.iter_mut() {
                    let _ = tx.send(ServerToClient::DoTick { tick: self.game_state.tick, inputs: self.player_inputs.clone() });
                }
                self.game_state.tick(&self.player_inputs);
                //println!("current tick: {}", self.game_state.tick);
                future::ready(())
            },
        }
    }
}

async fn handle_client_connection(server_tx: UnboundedSender<ServerInternalMsg>, websocket: WebSocket) {
    let (ws_tx, ws_rx) = websocket.split();
    let (s2c_tx, s2c_rx) = mpsc::unbounded_channel();
    let (c2s_tx, c2s_rx) = mpsc::unbounded_channel();
    tokio::task::spawn(s2c_rx.filter_map(|x| future::ready(match bincode::serialize(&x).map(Message::binary) {
        Ok(x) => Some(Ok(x)),
        Err(e) => { eprintln!("Error serializing {:?} to bincode: {:?}", x, e); None }
    })).forward(ws_tx));
    tokio::task::spawn(ws_rx.filter_map(|x| future::ready(x.ok())).filter_map(|x| future::ready(bincode::deserialize(x.as_bytes()).ok()))
        //.forward(c2s_tx)
        .for_each(move |x: ClientToServer| {
            //println!("Got c2s: {:?}", x);
            let _ = c2s_tx.send(x);
            future::ready(())
        })
    );
    let _ = server_tx.send(ServerInternalMsg::PlayerConnected(s2c_tx, c2s_rx));
}
