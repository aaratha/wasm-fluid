use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use web_sys::{window, HtmlCanvasElement, MouseEvent, WebGl2RenderingContext};

// Store state globally if needed
static mut GLO_CONTEXT: Option<WebGl2RenderingContext> = None;

#[wasm_bindgen(start)]
pub fn start() -> Result<(), JsValue> {
    let window = window().unwrap();
    let document = window.document().unwrap();
    let canvas = document.get_element_by_id("canvas").unwrap();
    let canvas: HtmlCanvasElement = canvas.dyn_into()?;

    let gl: WebGl2RenderingContext = canvas.get_context("webgl2")?.unwrap().dyn_into()?;

    gl.clear_color(1.0, 0.0, 0.0, 1.0);
    gl.clear(WebGl2RenderingContext::COLOR_BUFFER_BIT);

    // Store context globally
    unsafe {
        GLO_CONTEXT = Some(gl.clone());
    }

    // Add mouse click event listener
    let closure = Closure::wrap(Box::new(move |_event: MouseEvent| {
        // On click, change color randomly
        let r = js_sys::Math::random() as f32;
        let g = js_sys::Math::random() as f32;
        let b = js_sys::Math::random() as f32;

        unsafe {
            if let Some(ref gl) = GLO_CONTEXT {
                gl.clear_color(r, g, b, 1.0);
                gl.clear(WebGl2RenderingContext::COLOR_BUFFER_BIT);
            }
        }
    }) as Box<dyn FnMut(_)>);

    canvas.add_event_listener_with_callback("click", closure.as_ref().unchecked_ref())?;
    closure.forget(); // Leak memory intentionally to keep it alive

    Ok(())
}
