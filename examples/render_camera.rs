extern crate camera_thread;
#[macro_use]
extern crate gfx;
extern crate gfx_device_gl;
extern crate gfx_window_glutin;
extern crate glutin;

use gfx::handle;
use gfx::format;
use gfx::memory;
use gfx::state;
use gfx::traits::{Factory, FactoryExt};
use gfx::Resources;
use gfx::Device;
use gfx::texture;

pub type ColorFormat = format::Rgba8;
type DepthFormat = format::DepthStencil;

gfx_defines! {
    vertex Vertex {
        pos: [f32; 2] = "pos",
        texture_pos: [f32; 2] = "texture_pos",
    }

    pipeline pipe {
        vbuf: gfx::VertexBuffer<Vertex> = (),
        texture: gfx::TextureSampler<[u32; 4]> = "texture_camera",
        out: gfx::RenderTarget<ColorFormat> = "screen",
    }
}

const SCREEN: [Vertex; 4] = [
    Vertex { pos: [-1.0, 1.0], texture_pos: [0.0, 0.0] },
    Vertex { pos: [-1.0, -1.0], texture_pos: [0.0, 1.0] },
    Vertex { pos: [1.0, 1.0], texture_pos: [1.0, 0.0] },
    Vertex { pos: [1.0, -1.0], texture_pos: [1.0, 1.0] },
];

fn create_view<R, F, T>(factory: &mut F, texture: &handle::Texture<R, T::Surface>)
        -> gfx::handle::ShaderResourceView<R, T::View>
    where R: Resources, F: Factory<R>, T: format::TextureFormat {
    factory.view_texture_as_shader_resource::<T>(
        texture,
        (0, 0),
        format::Swizzle::new(),
    ).expect("failed to create view")
}

fn main() {
    let mut camera_config = camera_thread::Config::default();
    camera_config.image_format = camera_thread::ImageFormat::RGB;
    camera_config.resolution = (1280, 720);
    let camera_config = camera_config;
    let width = camera_config.resolution.0;
    let height = camera_config.resolution.1;

    let window_builder = glutin::WindowBuilder::new()
        .with_title("render_camera example".to_string())
        .with_dimensions(width, height)
        .with_vsync();

    let (window, mut device, mut factory, main_color, mut main_depth) =
    gfx_window_glutin::init::<ColorFormat, DepthFormat>(window_builder);
    let mut encoder: gfx::Encoder<_, _> = factory.create_command_buffer().into();
    let shader_set = factory.create_shader_set(include_bytes!("shader/render_camera.glslv"),
        include_bytes!("shader/render_camera.glslf")).expect("failed to create shader program");
    let pso = factory.create_pipeline_state(
        &shader_set,
        gfx::Primitive::TriangleStrip,
        state::Rasterizer::new_fill(),
        pipe::new(),
    ).expect("failed to create shader pipeline");

    let (vertex_buffer, slice) = factory.create_vertex_buffer_with_slice(&SCREEN, ());

    let texture = factory.create_texture::<format::R8_G8_B8_A8>(
        texture::Kind::D2(width as u16, height as u16, texture::AaMode::Single),
        1,
        gfx::SHADER_RESOURCE,
        memory::Usage::Dynamic,
        Some(format::ChannelType::Srgb),
    ).expect("failed to create texture");
    let texture_view = create_view::<_, _, [u8; 4]>(&mut factory, &texture);
    // we have no scaling, so take the linear one
    let sampler = factory.create_sampler_linear();

    let mut data = pipe::Data {
        vbuf: vertex_buffer,
        out: main_color,
        texture: (texture_view, sampler),
    };

    let mut camera_thread = camera_thread::CameraThread::new("/dev/video0", camera_config)
        .expect("failed to create camera thread");

    'main: loop {
        for event in window.poll_events() {
            match event {
                glutin::Event::KeyboardInput(_, _, Some(glutin::VirtualKeyCode::Escape)) |
                glutin::Event::Closed => break 'main,
                glutin::Event::Resized(_width, _height) => {
                    gfx_window_glutin::update_views(&window, &mut data.out, &mut main_depth);
                },
                _ => {},
            }
        }

        let frame = camera_thread.next_frame().expect("getting frame failed");
        if let Some(image) = frame {
            let mut buf = Vec::new();
            for rgb in image[..].chunks(3) {
                buf.push([rgb[0], rgb[1], rgb[2], 0xff]);
            }
            encoder.update_texture::<format::R8_G8_B8_A8, format::Srgba8>(&texture, None, texture.get_info().to_image_info(0), &buf)
                .expect("failed to update texture");

            encoder.draw(&slice, &pso, &data);

            encoder.flush(&mut device);
        }

        window.swap_buffers().unwrap();
        device.cleanup();
    }
}
