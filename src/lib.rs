use wasm_bindgen::prelude::*;
use web_sys::{window, HtmlCanvasElement, WebGl2RenderingContext};

#[wasm_bindgen(start)]
pub fn start() -> Result<(), JsValue> {
    let window = window().unwrap();
    let document = window.document().unwrap();
    let canvas = document.get_element_by_id("canvas").unwrap();
    let canvas: HtmlCanvasElement = canvas.dyn_into()?;

    let gl: WebGl2RenderingContext = canvas.get_context("webgl2")?.unwrap().dyn_into()?;

    gl.clear_color(1.0, 0.0, 0.0, 1.0);
    gl.clear(WebGl2RenderingContext::COLOR_BUFFER_BIT);

    Ok(())
}
