#![allow(static_mut_refs)]
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use web_sys::{window, HtmlCanvasElement, MouseEvent, WebGl2RenderingContext};

// Global mutable state (unsafe but simple for now)
static mut CIRCLE_X: f32 = 0.0;
static mut CIRCLE_Y: f32 = 0.0;
static mut IS_DRAGGING: bool = false;
static mut GLO_CONTEXT: Option<WebGl2RenderingContext> = None;
static mut RADIUS: f32 = 50.0;

#[wasm_bindgen(start)]
pub fn start() -> Result<(), JsValue> {
    let window = window().unwrap();
    let document = window.document().unwrap();
    let canvas = document.get_element_by_id("canvas").unwrap();
    let canvas: HtmlCanvasElement = canvas.dyn_into()?;

    resize_canvas(&canvas, &window);

    let gl: WebGl2RenderingContext = canvas.get_context("webgl2")?.unwrap().dyn_into()?;

    let width = canvas.width() as f32;
    let height = canvas.height() as f32;

    unsafe {
        CIRCLE_X = width / 2.0;
        CIRCLE_Y = height / 2.0;
        GLO_CONTEXT = Some(gl.clone());
    }

    draw_circle();

    // Mouse down: check if inside circle
    {
        let canvas_for_closure = canvas.clone();
        let window_for_inside = window.clone(); // <- keep original `window` untouched
        let closure = Closure::wrap(Box::new(move |event: MouseEvent| {
            let rect = canvas_for_closure.get_bounding_client_rect();
            let dpr = window_for_inside.device_pixel_ratio() as f32;
            let mouse_x = (event.client_x() as f32 - rect.left() as f32) * dpr;
            let mouse_y = (event.client_y() as f32 - rect.top() as f32) * dpr;

            unsafe {
                let dx = mouse_x - CIRCLE_X;
                let dy = mouse_y - CIRCLE_Y;
                if (dx * dx + dy * dy).sqrt() <= RADIUS {
                    IS_DRAGGING = true;
                    web_sys::console::log_1(&"Dragging started (inside mousedown)".into());
                }
                web_sys::console::log_1(&format!("Mouse down: ({}, {})", mouse_x, mouse_y).into());
                web_sys::console::log_1(
                    &format!("Circle: ({}, {}) r={}", CIRCLE_X, CIRCLE_Y, RADIUS).into(),
                );
            }
        }) as Box<dyn FnMut(_)>);
        canvas.add_event_listener_with_callback("mousedown", closure.as_ref().unchecked_ref())?;
        closure.forget();
    }

    // Mouse move: update position if dragging
    {
        let window_for_move = window.clone();
        let canvas_for_move = canvas.clone(); // <- keep original `canvas` untouched
        let closure = Closure::wrap(Box::new(move |event: MouseEvent| {
            let rect = canvas_for_move.get_bounding_client_rect();
            let dpr = window_for_move.device_pixel_ratio() as f32;
            let mouse_x = (event.client_x() as f32 - rect.left() as f32) * dpr;
            let mouse_y = (event.client_y() as f32 - rect.top() as f32) * dpr;
            // web_sys::console::log_1(&format!("Mouse move: ({}, {})", mouse_x, mouse_y).into());

            unsafe {
                if IS_DRAGGING {
                    web_sys::console::log_1(&"dragging".into());

                    CIRCLE_X = mouse_x;
                    CIRCLE_Y = mouse_y;
                    draw_circle();
                    // In mousemove handler:
                }
            }
        }) as Box<dyn FnMut(_)>);
        window.add_event_listener_with_callback("mousemove", closure.as_ref().unchecked_ref())?;
        closure.forget();
    }

    // Mouse up: stop dragging
    {
        let window_for_up = window.clone();
        let closure = Closure::wrap(Box::new(move |_event: MouseEvent| unsafe {
            IS_DRAGGING = false;
        }) as Box<dyn FnMut(_)>);
        window.add_event_listener_with_callback("mouseup", closure.as_ref().unchecked_ref())?;
        closure.forget();
    }

    // Resize event: redraw circle
    {
        let canvas_for_resize = canvas.clone();
        let window_for_resize = window.clone();

        let closure = Closure::wrap(Box::new(move |_event: web_sys::Event| {
            resize_canvas(&canvas_for_resize, &window_for_resize);
            unsafe {
                draw_circle();
            }
        }) as Box<dyn FnMut(_)>);

        window.add_event_listener_with_callback("resize", closure.as_ref().unchecked_ref())?;
        closure.forget();
    }

    Ok(())
}

/// Draw a filled circle using triangle fan
fn draw_circle() {
    const SEGMENTS: usize = 100;

    unsafe {
        if let Some(gl) = &GLO_CONTEXT {
            // Get current canvas size
            let canvas = gl
                .canvas()
                .unwrap()
                .dyn_into::<HtmlCanvasElement>()
                .unwrap();
            let width = canvas.width() as f32;
            let height = canvas.height() as f32;

            gl.viewport(0, 0, width as i32, height as i32);
            gl.clear_color(1.0, 1.0, 1.0, 1.0);
            gl.clear(WebGl2RenderingContext::COLOR_BUFFER_BIT);

            let mut vertices: Vec<f32> = Vec::with_capacity((SEGMENTS + 2) * 2);

            // Keep everything in screen space
            let center_x = CIRCLE_X;
            let center_y = CIRCLE_Y;
            let radius = RADIUS;

            // Center vertex
            vertices.push(center_x);
            vertices.push(center_y);

            // Perimeter vertices
            for i in 0..=SEGMENTS {
                let angle = i as f32 / SEGMENTS as f32 * std::f32::consts::PI * 2.0;
                let x = center_x + radius * angle.cos();
                let y = center_y + radius * angle.sin();
                vertices.push(x);
                vertices.push(y);
            }

            let buffer = gl.create_buffer().unwrap();
            gl.bind_buffer(WebGl2RenderingContext::ARRAY_BUFFER, Some(&buffer));

            let vert_array = js_sys::Float32Array::view(&vertices);
            gl.buffer_data_with_array_buffer_view(
                WebGl2RenderingContext::ARRAY_BUFFER,
                &vert_array,
                WebGl2RenderingContext::STATIC_DRAW,
            );

            let vert_shader = compile_shader(
                gl,
                WebGl2RenderingContext::VERTEX_SHADER,
                r#"
                attribute vec2 position;
                uniform vec2 u_resolution;

                void main() {
                    // Convert from pixels to [0, 1]
                    vec2 zeroToOne = position / u_resolution;

                    // Convert from [0,1] to [-1,1]
                    vec2 clipSpace = zeroToOne * 2.0 - 1.0;

                    // Flip Y because WebGL has +Y going up
                    gl_Position = vec4(clipSpace * vec2(1, -1), 0.0, 1.0);
                }
                "#,
            )
            .unwrap();

            let frag_shader = compile_shader(
                gl,
                WebGl2RenderingContext::FRAGMENT_SHADER,
                r#"
                void main() {
                    gl_FragColor = vec4(0.0, 0.5, 1.0, 1.0);
                }
                "#,
            )
            .unwrap();

            let program = link_program(gl, &vert_shader, &frag_shader).unwrap();
            gl.use_program(Some(&program));

            let res_location = gl.get_uniform_location(&program, "u_resolution");
            // from WebGL2RenderingContext.uniform[1234][uif][v]()
            gl.uniform2f(res_location.as_ref(), width, height);

            let pos_attrib = gl.get_attrib_location(&program, "position") as u32;
            gl.enable_vertex_attrib_array(pos_attrib);
            gl.vertex_attrib_pointer_with_i32(
                pos_attrib,
                2,
                WebGl2RenderingContext::FLOAT,
                false,
                0,
                0,
            );

            gl.draw_arrays(
                WebGl2RenderingContext::TRIANGLE_FAN,
                0,
                (SEGMENTS + 2) as i32,
            );
        }
    }
}

fn compile_shader(
    gl: &WebGl2RenderingContext,
    shader_type: u32,
    src: &str,
) -> Result<web_sys::WebGlShader, String> {
    let shader = gl
        .create_shader(shader_type)
        .ok_or("Unable to create shader")?;
    gl.shader_source(&shader, src);
    gl.compile_shader(&shader);

    if gl
        .get_shader_parameter(&shader, WebGl2RenderingContext::COMPILE_STATUS)
        .as_bool()
        .unwrap_or(false)
    {
        Ok(shader)
    } else {
        Err(gl
            .get_shader_info_log(&shader)
            .unwrap_or_else(|| "Unknown error compiling shader".into()))
    }
}

fn link_program(
    gl: &WebGl2RenderingContext,
    vs: &web_sys::WebGlShader,
    fs: &web_sys::WebGlShader,
) -> Result<web_sys::WebGlProgram, String> {
    let program = gl.create_program().ok_or("Unable to create program")?;
    gl.attach_shader(&program, vs);
    gl.attach_shader(&program, fs);
    gl.link_program(&program);

    if gl
        .get_program_parameter(&program, WebGl2RenderingContext::LINK_STATUS)
        .as_bool()
        .unwrap_or(false)
    {
        Ok(program)
    } else {
        Err(gl
            .get_program_info_log(&program)
            .unwrap_or_else(|| "Unknown error linking program".into()))
    }
}

fn resize_canvas(canvas: &HtmlCanvasElement, window: &web_sys::Window) {
    let dpr = window.device_pixel_ratio();

    // CSS size
    let width_css = canvas.client_width();
    let height_css = canvas.client_height();

    // Actual pixel size = CSS size * DPR
    let width_px = (width_css as f64 * dpr).round() as u32;
    let height_px = (height_css as f64 * dpr).round() as u32;

    // Set the canvas "drawing buffer" size in pixels
    if canvas.width() != width_px || canvas.height() != height_px {
        canvas.set_width(width_px);
        canvas.set_height(height_px);
    }

    unsafe {
        CIRCLE_X = (width_px as f32) / 2.0;
        CIRCLE_Y = (height_px as f32) / 2.0;
    }
}
