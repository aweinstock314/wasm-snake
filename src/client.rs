#![cfg(feature="client-deps")]

use web_sys::MessageEvent;
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;

#[wasm_bindgen]
pub fn wasm_main() -> Result<(), JsValue> {
    #[global_allocator]
    static ALLOC: wee_alloc::WeeAlloc = wee_alloc::WeeAlloc::INIT;

    web_sys::console::log_1(&JsValue::from_str("Hello to the console!"));

    let window = web_sys::window().unwrap();
    let document = window.document().unwrap();
    let pre = document.get_element_by_id("logging_pre").unwrap();
    pre.set_text_content(Some("Hello, world!"));

    let onmessage_closure = Closure::wrap(Box::new(move |msg: MessageEvent| {
        if let Some(data) = msg.data().as_string() {
            pre.set_text_content(Some(&data));
        }
    }) as Box<dyn FnMut(MessageEvent)>);

    let ws = web_sys::WebSocket::new("ws://127.0.0.1:8000/client_connection").unwrap();
    ws.set_onmessage(onmessage_closure.as_ref().dyn_ref());

    onmessage_closure.forget();

    Ok(())
}
