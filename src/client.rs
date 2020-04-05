#![cfg(feature="client-deps")]

use wasm_bindgen::prelude::*;



#[wasm_bindgen]
pub fn wasm_main() -> Result<(), JsValue> {
    #[global_allocator]
    static ALLOC: wee_alloc::WeeAlloc = wee_alloc::WeeAlloc::INIT;

    web_sys::console::log_1(&JsValue::from_str("Hello to the console!"));

    let window = web_sys::window().unwrap();
    let document = window.document().unwrap();
    let pre = document.get_element_by_id("logging_pre").unwrap();
    pre.set_text_content(Some("Hello, world!"));

    Ok(())
}
