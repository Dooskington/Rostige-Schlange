extern crate gl;
extern crate libc;
extern crate glutin;
extern crate image;
extern crate nalgebra;
extern crate alga;

use std::mem;
use std::ptr;
use std::ffi::CStr;
use glutin::GlContext;
use gl::types::*;
use image::RgbaImage;
use nalgebra::*;

const VS_SRC: &'static [u8] = b"
#version 150 core

uniform mat4 modelViewProjection;

in vec2 position;
in vec3 color;
in vec2 uv;

out vec3 Color;
out vec2 TexCoord;

void main()
{
    Color = color;
    TexCoord = uv;
    gl_Position = modelViewProjection * vec4(position, 0.0, 1.0);
}
\0";

const FS_SRC: &'static [u8] = b"
#version 150 core

uniform sampler2D tex;

in vec3 Color;
in vec2 TexCoord;

out vec4 outColor;

void main()
{
    outColor = texture(tex, TexCoord) * vec4(Color, 1.0);
}
\0";

pub const CELL_WIDTH: u32 = 8;
pub const CELL_HEIGHT: u32 = 16;

pub struct Window {
    pub events_loop: glutin::EventsLoop,
    pub gl_window: glutin::GlWindow,
    pub width: u32,
    pub height: u32,
    pub is_close_requested: bool
}

impl Window {
    pub fn new(title: &str, width: u32, height: u32) -> Window {
        let window = glutin::WindowBuilder::new()
            .with_title(title)
            .with_dimensions(width, height)
            .with_min_dimensions(width, height)
            .with_max_dimensions(width, height);

        let context = glutin::ContextBuilder::new()
            .with_vsync(true);

        let events_loop = glutin::EventsLoop::new();
        let gl_window = glutin::GlWindow::new(window, context, &events_loop).unwrap();

        unsafe {
            gl_window.make_current().unwrap();
        }

        Window {
            events_loop,
            gl_window,
            width,
            height,
            is_close_requested: false
        }
    }
}

pub fn resize_window(window: &mut Window, width: u32, height: u32) {
    window.gl_window.resize(width, height);
    window.width = width;
    window.height = height;
}

pub struct Renderer {
    pub cols: u32,
    pub rows: u32,
    cells: Vec<Sprite>,
    needs_rebuild: bool,
    vao_id: GLuint,
    vbo_id: GLuint,
    ebo_id: GLuint,
    vertex_data: Vec<f32>,
    element_data: Vec<u32>
}

impl Renderer {
    pub fn new(window: &Window) -> Renderer {
        gl::load_with(|symbol| window.gl_window.get_proc_address(symbol) as *const _);

        let cols: u32 = window.width / CELL_WIDTH;
        let rows: u32 = window.height / CELL_HEIGHT;

        let mut cells: Vec<Sprite> = Vec::new();
        cells.resize((cols * rows) as usize, SPRITE_NONE);

        let mut vao: GLuint = 0;
        let mut vbo: GLuint = 0;
        let mut ebo: GLuint = 0;

        unsafe {
            gl::Enable(gl::BLEND);
            gl::BlendFunc(gl::SRC_ALPHA, gl::ONE_MINUS_SRC_ALPHA);
            gl::ClearColor(0.0, 0.0, 0.0, 1.0);

            gl::GenVertexArrays(1, &mut vao);
            gl::BindVertexArray(vao);

            gl::GenBuffers(1, &mut ebo);
            gl::BindBuffer(gl::ELEMENT_ARRAY_BUFFER, ebo);

            gl::GenBuffers(1, &mut vbo);
            gl::BindBuffer(gl::ARRAY_BUFFER, vbo);

            let vertex_shader: GLuint = compile_shader(gl::VERTEX_SHADER, VS_SRC);
            let fragment_shader: GLuint = compile_shader(gl::FRAGMENT_SHADER, FS_SRC);

            let shader_program: GLuint = gl::CreateProgram();
            gl::AttachShader(shader_program, vertex_shader);
            gl::AttachShader(shader_program, fragment_shader);
            gl::LinkProgram(shader_program);
            gl::UseProgram(shader_program);

            let position_attribute: GLint = gl::GetAttribLocation(shader_program, b"position\0".as_ptr() as *const _);
            gl::VertexAttribPointer(position_attribute as GLuint, 2, gl::FLOAT, 0,
                                    7 * mem::size_of::<f32>() as GLsizei,
                                    ptr::null());

            let color_attribute: GLint = gl::GetAttribLocation(shader_program, b"color\0".as_ptr() as *const _);
            gl::VertexAttribPointer(color_attribute as GLuint, 3, gl::FLOAT, 0,
                                    7 * mem::size_of::<f32>() as GLsizei,
                                    (2 * mem::size_of::<f32>()) as *const _);

            let uv_attribute: GLint = gl::GetAttribLocation(shader_program, b"uv\0".as_ptr() as *const _);
            gl::VertexAttribPointer(uv_attribute as GLuint, 2, gl::FLOAT, 0,
                                    7 * mem::size_of::<f32>() as GLsizei,
                                    (5 * mem::size_of::<f32>()) as *const _);

            gl::EnableVertexAttribArray(position_attribute as GLuint);
            gl::EnableVertexAttribArray(color_attribute as GLuint);
            gl::EnableVertexAttribArray(uv_attribute as GLuint);

            let image: RgbaImage = image::open("font.png")
                .expect("Failed to open image!")
                .to_rgba();

            let width: GLint = image.width() as GLint;
            let height: GLint = image.height() as GLint;
            let pixels: Vec<u8> = image.into_raw();

            let mut texture_id: GLuint = 0;
            gl::GenTextures(1, &mut texture_id);
            gl::BindTexture(gl::TEXTURE_2D, texture_id);
            gl::TexImage2D(gl::TEXTURE_2D,
                            0,
                            gl::RGBA8 as GLint,
                            width,
                            height,
                            0,
                            gl::RGBA,
                            gl::UNSIGNED_BYTE,
                            pixels.as_ptr() as *const _);

            gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_WRAP_S, gl::REPEAT as i32);
            gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_WRAP_T, gl::REPEAT as i32);
            gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_MIN_FILTER, gl::NEAREST as i32);
            gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_MAG_FILTER, gl::NEAREST as i32);
            gl::GenerateMipmap(gl::TEXTURE_2D);

            let translation: Vector3<f32> = Vector3::new(0.0, 0.0, 0.0);

            let model: Matrix4<f32> = Isometry3::new(translation, nalgebra::zero()).to_homogeneous();
            let view: Matrix4<f32> = Isometry3::new(Vector3::new(0.0, 0.0, -1.0), nalgebra::zero()).to_homogeneous();
            let projection: Matrix4<f32> = Orthographic3::new(0.0, window.width as f32, 0.0, window.height as f32, 0.1, 1000.0).unwrap();
            let model_view_projection = projection * model * view;

            let uni_model_view_projection = gl::GetUniformLocation(shader_program, b"modelViewProjection\0".as_ptr() as *const _);
            gl::UniformMatrix4fv(uni_model_view_projection, 1, gl::FALSE, model_view_projection.as_slice().as_ptr() as *const _);
        }

        let version = unsafe {
            let data = CStr::from_ptr(gl::GetString(gl::VERSION) as *const _).to_bytes().to_vec();
            String::from_utf8(data).unwrap()
        };

        println!("OpenGL version {}", version);

        Renderer {
            cols,
            rows,
            cells,
            needs_rebuild: false,
            vao_id: vao,
            vbo_id: vbo,
            ebo_id: ebo,
            vertex_data: Vec::new(),
            element_data: Vec::new()
        }
    }

    pub fn clear_cells(&mut self) {
        self.cells.clear();
        self.cells.resize((self.cols * self.rows) as usize, SPRITE_NONE);
    }
}

pub fn draw_cell(renderer: &mut Renderer, x: i32, y: i32, sprite: Sprite) {
    if (x < 0) || (y < 0) || (x as u32 >= renderer.cols) || (y as u32 >= renderer.rows) {
        return;
    }

    let index: usize = ((y as u32 * renderer.cols) + x as u32) as usize;
    if renderer.cells[index] != sprite {
        renderer.cells[index] = sprite;
        renderer.needs_rebuild = true;
    }
}

pub fn draw_string(renderer: &mut Renderer, x: i32, y: i32, string: &str) {
    let mut x: i32 = x;

    for c in string.chars() {
        let sprite: Sprite = Sprite::new(c, COLOR_WHITE);
        draw_cell(renderer, x, y, sprite);
        x += 1;
    }
}

pub fn draw_box(renderer: &mut Renderer, x: i32, y: i32, width: u32, height: u32) {
    draw_cell(renderer, x, y, SPRITE_BOX_BOTTOM_LEFT); // Bottom left
    draw_cell(renderer, x + (width as i32 - 1), y, SPRITE_BOX_BOTTOM_RIGHT); // Bottom right
    draw_cell(renderer, x, y + (height as i32 - 1), SPRITE_BOX_TOP_LEFT);  // Top left
    draw_cell(renderer, x + (width as i32 - 1), y + (height as i32 - 1), SPRITE_BOX_TOP_RIGHT); // Top right

    for i in (x + 1)..(x + width as i32 - 1) {
        draw_cell(renderer, i, y, SPRITE_BOX_HORIZONTAL); // Bottom
        draw_cell(renderer, i, y + (height as i32 - 1), SPRITE_BOX_HORIZONTAL); // Top
    }

    for i in (y + 1)..(y + height as i32 - 1) {
        draw_cell(renderer, x, i, SPRITE_BOX_VERTICAL); // Left
        draw_cell(renderer, x + (width as i32 - 1), i, SPRITE_BOX_VERTICAL); // Right
    }
}

pub fn clear(renderer: &mut Renderer) {
    renderer.clear_cells();
    renderer.needs_rebuild = true;
}

pub fn render(renderer: &mut Renderer) {
    unsafe {
        if renderer.needs_rebuild {
            upload(renderer);
        }

        gl::Clear(gl::COLOR_BUFFER_BIT);
        gl::DrawElements(gl::TRIANGLES, renderer.element_data.len() as i32, gl::UNSIGNED_INT, ptr::null());
    }
}

pub fn display(window: &Window) {
    window.gl_window.swap_buffers().unwrap();
}

fn compile_shader(shader_type: GLenum, source: &[u8]) -> GLuint {
    unsafe {
        let shader_id: GLuint = gl::CreateShader(shader_type);
        gl::ShaderSource(shader_id, 1, [source.as_ptr() as *const _].as_ptr(), ptr::null());
        gl::CompileShader(shader_id);

        let mut status: GLint = gl::TRUE as GLint;
        gl::GetShaderiv(shader_id, gl::COMPILE_STATUS, &mut status as *mut GLint);
        if status == (gl::FALSE as GLint) {
            panic!("Shader compilation failed!");
        }

        return shader_id;
    }
}

fn upload(renderer: &mut Renderer) {
    renderer.vertex_data.clear();
    renderer.element_data.clear();

    // Construct render mesh
    for row in 0..renderer.rows {
        for col in 0..renderer.cols {
            let index: usize = ((row * renderer.cols) + col) as usize;
            let cell: Sprite = renderer.cells[index];

            if cell.graphic == ' ' {
                continue;
            }

            let vertex_count: u32 = renderer.vertex_data.len() as u32 / 7;
            let x_offset: f32 = (col * CELL_WIDTH) as f32;
            let y_offset: f32 = (row * CELL_HEIGHT) as f32;

            let cols: u8 = 16;

            let ascii: u8 = cell.graphic as u8;
            let sprite_col: u8 = ascii % cols;
            let sprite_row: u8 = ascii / cols;
            let sprite_width: f32 = CELL_WIDTH as f32 / 128.0;
            let sprite_height: f32 = CELL_HEIGHT as f32 / 256.0;
            let u: f32 = sprite_col as f32 * sprite_width;
            let v: f32 = sprite_row as f32 * sprite_height;

            let r: f32 = cell.color.r;
            let g: f32 = cell.color.g;
            let b: f32 = cell.color.b;
            //let a: f32 = cell.color.a;

            let new_vertices: [f32; 28] = [
                // Top left
                x_offset, y_offset + CELL_HEIGHT as f32, r, g, b, u, v,
                // Top right
                x_offset + CELL_WIDTH as f32, y_offset + CELL_HEIGHT as f32, r, g, b, u + sprite_width, v,
                // Bottom right
                x_offset + CELL_WIDTH as f32, y_offset, r, g, b, u + sprite_width, v + sprite_height,
                // Bottom left
                x_offset, y_offset, r, g, b, u, v + sprite_height
            ];

            let new_elements: [u32; 6] = [
                vertex_count, vertex_count + 1, vertex_count + 2,
                vertex_count + 2, vertex_count + 3, vertex_count
            ];

            renderer.vertex_data.extend_from_slice(&new_vertices);
            renderer.element_data.extend_from_slice(&new_elements);
        }
    }

    // Upload vertices

    unsafe {
        gl::BindBuffer(gl::ARRAY_BUFFER, renderer.vbo_id);
        gl::BufferData(gl::ARRAY_BUFFER,
                       (renderer.vertex_data.len() * mem::size_of::<f32>()) as gl::types::GLsizeiptr,
                       renderer.vertex_data.as_ptr() as *const _, gl::STATIC_DRAW);

        gl::BindBuffer(gl::ELEMENT_ARRAY_BUFFER, renderer.ebo_id);
        gl::BufferData(gl::ELEMENT_ARRAY_BUFFER,
                       (renderer.element_data.len() * mem::size_of::<u32>()) as gl::types::GLsizeiptr,
                       renderer.element_data.as_ptr() as *const _, gl::STATIC_DRAW);
    }

    renderer.needs_rebuild = false;
}

pub const SPRITE_NONE: Sprite = Sprite { graphic: ' ', color: COLOR_WHITE };
pub const SPRITE_BOX_BOTTOM_LEFT: Sprite = Sprite { graphic: 192 as char, color: COLOR_WHITE };
pub const SPRITE_BOX_BOTTOM_RIGHT: Sprite = Sprite { graphic: 217 as char, color: COLOR_WHITE };
pub const SPRITE_BOX_TOP_LEFT: Sprite = Sprite { graphic: 218 as char, color: COLOR_WHITE };
pub const SPRITE_BOX_TOP_RIGHT: Sprite = Sprite { graphic: 191 as char, color: COLOR_WHITE };
pub const SPRITE_BOX_HORIZONTAL: Sprite = Sprite { graphic: 196 as char, color: COLOR_WHITE };
pub const SPRITE_BOX_VERTICAL: Sprite = Sprite { graphic: 179 as char, color: COLOR_WHITE };

#[derive(Copy, Clone, PartialEq)]
pub struct Sprite {
    pub graphic: char,
    pub color: Color
}

impl Sprite {
    pub fn new(graphic: char, color: Color) -> Sprite {
        Sprite {
            graphic,
            color
        }
    }
}

pub const COLOR_BLACK: Color = Color { r: 0.0, g: 0.0, b: 0.0, a: 1.0 };
pub const COLOR_WHITE: Color = Color { r: 1.0, g: 1.0, b: 1.0, a: 1.0 };
pub const COLOR_GRAY: Color = Color { r: 0.4, g: 0.4, b: 0.4, a: 1.0 };
pub const COLOR_RED: Color = Color { r: 1.0, g: 0.0, b: 0.0, a: 1.0 };
pub const COLOR_GREEN: Color = Color { r: 0.0, g: 1.0, b: 0.0, a: 1.0 };
pub const COLOR_BLUE: Color = Color { r: 0.0, g: 0.0, b: 1.0, a: 1.0 };

#[derive(Copy, Clone, PartialEq)]
pub struct Color {
    r: f32,
    g: f32,
    b: f32,
    a: f32
}

impl Color {
    pub fn new(r: u8, g: u8, b: u8, a: u8) -> Color {
        Color {
            r: r as f32 / 255.0,
            g: g as f32 / 255.0,
            b: b as f32 / 255.0,
            a: a as f32 / 255.0
        }
    }
}