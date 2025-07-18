#![allow(static_mut_refs)]
use console_error_panic_hook;
use js_sys::Float32Array;
use js_sys::Math::random;
use std::cell::RefCell;
use std::rc::Rc;
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use web_sys::{window, HtmlCanvasElement, MouseEvent, WebGl2RenderingContext, WebGlShader};

// Global mutable state (unsafe but simple for now)
static mut CIRCLE_X: f32 = 0.0;
static mut CIRCLE_Y: f32 = 0.0;
static mut TARGET_X: f32 = 0.0;
static mut TARGET_Y: f32 = 0.0;

static mut IS_DRAGGING: bool = false;
static mut GLO_CONTEXT: Option<WebGl2RenderingContext> = None;
static mut RADIUS: f32 = 50.0;

const PARTICLE_GRID_SIZE: usize = 64; // 64x64 = 4096 particles
const PARTICLE_COUNT: usize = PARTICLE_GRID_SIZE * PARTICLE_GRID_SIZE;
thread_local! {
    static PARTICLES: RefCell<Vec<Particle>> = RefCell::new(vec![]);
}

#[derive(Clone, Copy)]
struct Particle {
    x: f32,
    y: f32,
    vx: f32,
    vy: f32,
}

#[wasm_bindgen(start)]
pub fn start() -> Result<(), JsValue> {
    console_error_panic_hook::set_once();
    let window = window().unwrap();
    let document = window.document().unwrap();
    let canvas = document
        .get_element_by_id("canvas")
        .unwrap()
        .dyn_into::<web_sys::HtmlCanvasElement>()
        .unwrap();

    resize_canvas(&canvas, &window);

    let gl = canvas
        .get_context("webgl2")?
        .unwrap()
        .dyn_into::<WebGl2RenderingContext>()?;

    let width = canvas.width() as f32;
    let height = canvas.height() as f32;

    let particles = generate_particles(width, height);
    PARTICLES.with(|p| *p.borrow_mut() = particles);

    unsafe {
        CIRCLE_X = width / 2.0;
        CIRCLE_Y = height / 2.0;
        GLO_CONTEXT = Some(gl.clone());
    }

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

                    TARGET_X = mouse_x;
                    TARGET_Y = mouse_y;
                    // In mousemove handler:
                }
            }
        }) as Box<dyn FnMut(_)>);
        window.add_event_listener_with_callback("mousemove", closure.as_ref().unchecked_ref())?;
        closure.forget();
    }

    // Mouse up: stop dragging
    {
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
        }) as Box<dyn FnMut(_)>);

        window.add_event_listener_with_callback("resize", closure.as_ref().unchecked_ref())?;
        closure.forget();
    }

    start_render_loop(&gl);

    Ok(())
}

/// Draw a filled circle using triangle fan
fn _draw_circle() {
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
                r#"#version 300 es
                precision highp float;

                in vec2 position;
                uniform vec2 u_resolution;

                void main() {
                    vec2 zeroToOne = position / u_resolution;
                    vec2 clipSpace = zeroToOne * 2.0 - 1.0;
                    gl_Position = vec4(clipSpace * vec2(1, -1), 0.0, 1.0);
                }
                "#,
            )
            .unwrap();

            let frag_shader = compile_shader(
                gl,
                WebGl2RenderingContext::FRAGMENT_SHADER,
                r#"#version 300 es

                precision highp float;

                uniform sampler2D u_particles;
                out vec4 outColor;

                void main() {
                    ivec2 coord = ivec2(gl_FragCoord.xy);
                    vec4 particle = texelFetch(u_particles, coord, 0);

                    // Just color by velocity
                    outColor = vec4(0.5 + particle.z * 0.05, 0.5 + particle.w * 0.05, 1.0, 1.0);
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
    source: &str,
) -> Result<WebGlShader, String> {
    let shader = gl
        .create_shader(shader_type)
        .ok_or_else(|| String::from("Unable to create shader object"))?;

    gl.shader_source(&shader, source);
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
            .unwrap_or_else(|| String::from("Unknown error")))
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

fn _lerp(start: f32, end: f32, t: f32) -> f32 {
    start + t * (end - start)
}

fn start_render_loop(gl: &WebGl2RenderingContext) {
    let gl = gl.clone();
    let window = web_sys::window().unwrap();

    let f: Rc<RefCell<Option<Closure<dyn FnMut()>>>> = Rc::new(RefCell::new(None));
    let g = f.clone();

    let closure = Closure::wrap(Box::new({
        let window = window.clone();
        move || {
            let canvas = gl
                .canvas()
                .unwrap()
                .dyn_into::<HtmlCanvasElement>()
                .unwrap();
            let width = canvas.width() as f32;
            let height = canvas.height() as f32;

            // Update particles
            PARTICLES.with(|particles| {
                let mut ps = particles.borrow_mut();
                for p in ps.iter_mut() {
                    p.x += p.vx;
                    p.y += p.vy;

                    // Bounce off edges
                    if p.x <= 0.0 || p.x >= width {
                        p.vx *= -1.0;
                    }
                    if p.y <= 0.0 || p.y >= height {
                        p.vy *= -1.0;
                    }
                }
            });

            //draw_circle();
            draw_particles(&gl, width, height);

            window
                .request_animation_frame(f.borrow().as_ref().unwrap().as_ref().unchecked_ref())
                .unwrap();
        }
    }) as Box<dyn FnMut()>);

    *g.borrow_mut() = Some(closure);

    // Start animation frame with closure reference
    window
        .request_animation_frame(g.borrow().as_ref().unwrap().as_ref().unchecked_ref())
        .unwrap();
}

fn rand_range(min: f32, max: f32) -> f32 {
    min + random() as f32 * (max - min)
}

fn generate_particles(width: f32, height: f32) -> Vec<Particle> {
    (0..PARTICLE_COUNT)
        .map(|_| Particle {
            x: rand_range(0.0, width),
            y: rand_range(0.0, height),
            vx: rand_range(-1.0, 1.0),
            vy: rand_range(-1.0, 1.0),
        })
        .collect()
}

fn draw_particles(gl: &WebGl2RenderingContext, width: f32, height: f32) {
    let mut data = vec![];

    PARTICLES.with(|particles| {
        for p in particles.borrow().iter() {
            data.push(p.x);
            data.push(p.y);
        }
    });

    let vert_array = Float32Array::from(data.as_slice());

    let buffer = gl.create_buffer().unwrap();
    gl.bind_buffer(WebGl2RenderingContext::ARRAY_BUFFER, Some(&buffer));
    gl.buffer_data_with_array_buffer_view(
        WebGl2RenderingContext::ARRAY_BUFFER,
        &vert_array,
        WebGl2RenderingContext::DYNAMIC_DRAW,
    );

    let vert_shader = compile_shader(
        gl,
        WebGl2RenderingContext::VERTEX_SHADER,
        r#"#version 300 es
        precision mediump float;
        in vec2 position;
        uniform vec2 u_resolution;

        void main() {
            vec2 zeroToOne = position / u_resolution;
            vec2 clipSpace = zeroToOne * 2.0 - 1.0;
            gl_Position = vec4(clipSpace * vec2(1, -1), 0.0, 1.0);
            gl_PointSize = 4.0;
        }"#,
    )
    .unwrap();

    let frag_shader = compile_shader(
        gl,
        WebGl2RenderingContext::FRAGMENT_SHADER,
        r#"#version 300 es
        precision mediump float;
        out vec4 outColor;

        void main() {
            outColor = vec4(0, 0, 1, 1);
        }"#,
    )
    .unwrap();

    let program = link_program(gl, &vert_shader, &frag_shader).unwrap();
    gl.use_program(Some(&program));

    let pos_attrib = gl.get_attrib_location(&program, "position") as u32;
    gl.enable_vertex_attrib_array(pos_attrib);
    gl.vertex_attrib_pointer_with_i32(pos_attrib, 2, WebGl2RenderingContext::FLOAT, false, 0, 0);

    let res_location = gl.get_uniform_location(&program, "u_resolution").unwrap();
    gl.uniform2f(Some(&res_location), width, height);

    gl.clear_color(0.9, 0.9, 0.9, 1.0);
    gl.clear(WebGl2RenderingContext::COLOR_BUFFER_BIT);

    gl.draw_arrays(WebGl2RenderingContext::POINTS, 0, (data.len() / 2) as i32);
}
