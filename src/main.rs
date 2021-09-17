// based on https://github.com/hasenbanck/egui_example
use std::iter;
use std::time::Instant;

use egui::FontDefinitions;
use egui_wgpu_backend::{RenderPass, ScreenDescriptor};
use egui_winit_platform::{Platform, PlatformDescriptor};
use futures_lite::future::block_on;
use winit::event::Event::*;
use winit::event_loop::ControlFlow;

const INITIAL_WIDTH: u32 = 400;
const INITIAL_HEIGHT: u32 = 400;

/// A simple egui + wgpu + winit based example.
fn main() {
    let event_loop = winit::event_loop::EventLoop::new();
    let window = winit::window::WindowBuilder::new()
        .with_decorations(true)
        .with_resizable(true)
        .with_transparent(false)
        .with_title("egui-wgpu_winit example")
        .with_inner_size(winit::dpi::PhysicalSize {
            width: INITIAL_WIDTH,
            height: INITIAL_HEIGHT,
        })
        .build(&event_loop)
        .unwrap();

    let instance = wgpu::Instance::new(wgpu::BackendBit::VULKAN);
    let surface = unsafe { instance.create_surface(&window) };

    let adapter = instance
        .enumerate_adapters(wgpu::BackendBit::VULKAN)
        .filter(|adapter| {
            let info = adapter.get_info();
            println!("{:?}", info);
            info.name.to_lowercase().contains("intel")
        })
        .next()
        .expect("No intel adapters available");

    let (mut device, mut queue) = block_on(adapter.request_device(
        &wgpu::DeviceDescriptor {
            features: wgpu::Features::default(),
            limits: wgpu::Limits::default(),
            label: None,
        },
        None,
    ))
    .unwrap();

    let size = window.inner_size();
    //  let surface_format = surface.get_preferred_format(&adapter).unwrap();
    let surface_format = adapter.get_swap_chain_preferred_format(&surface).unwrap();
    let surface_config = wgpu::SwapChainDescriptor {
        usage: wgpu::TextureUsage::RENDER_ATTACHMENT,
        format: surface_format,
        width: size.width as u32,
        height: size.height as u32,
        present_mode: wgpu::PresentMode::Mailbox,
    };
    let swap_chain = device.create_swap_chain(&surface, &surface_config);

    // We use the egui_winit_platform crate as the platform.
    let mut platform = Platform::new(PlatformDescriptor {
        physical_width: size.width as u32,
        physical_height: size.height as u32,
        scale_factor: window.scale_factor(),
        font_definitions: FontDefinitions::default(),
        style: Default::default(),
    });

    // We use the egui_wgpu_backend crate as the render backend.
    let mut egui_rpass = RenderPass::new(&device, surface_format, 1);

    let start_time = Instant::now();
    event_loop.run(move |event, _, control_flow| {
        // Pass the winit events to the platform integration.
        platform.handle_event(&event);

        match event {
            RedrawRequested(..) => {
                platform.update_time(start_time.elapsed().as_secs_f64());

                let output_frame = match swap_chain.get_current_frame() {
                    Ok(frame) => frame,
                    Err(e) => {
                        eprintln!("Dropped frame with error: {}", e);
                        return;
                    }
                };

                // Begin to draw the UI frame.
                platform.begin_frame();

                // End the UI frame. We could now handle the output and draw the UI with the backend.
                let (_output, paint_commands) = platform.end_frame();
                let paint_jobs = platform.context().tessellate(paint_commands);

                let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
                    label: Some("encoder"),
                });

                // Upload all resources for the GPU.
                let screen_descriptor = ScreenDescriptor {
                    physical_width: surface_config.width,
                    physical_height: surface_config.height,
                    scale_factor: window.scale_factor() as f32,
                };
                egui_rpass.update_texture(&device, &queue, &platform.context().texture());
                egui_rpass.update_user_textures(&device, &queue);
                egui_rpass.update_buffers(&mut device, &mut queue, &paint_jobs, &screen_descriptor);

                // Record all render passes.
                egui_rpass.execute(
                    &mut encoder,
                    &output_frame.output.view,
                    &paint_jobs,
                    &screen_descriptor,
                    Some(wgpu::Color::BLACK),
                );

                // Submit the commands.
                queue.submit(iter::once(encoder.finish()));
                *control_flow = ControlFlow::Poll;
            }
            MainEventsCleared => {
                window.request_redraw();
            }
            WindowEvent { event, .. } => match event {
                winit::event::WindowEvent::Resized(_size) => {
                    //surface_config.width = size.width;
                    //surface_config.height = size.height;
                    //surface.configure(&device, &surface_config);
                }
                winit::event::WindowEvent::CloseRequested => {
                    *control_flow = ControlFlow::Exit;
                }
                _ => {}
            },
            _ => (),
        }
    });
}
