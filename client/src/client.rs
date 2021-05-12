// Copyright James Carl (C) 2020-2021
// AGPL-3.0-or-later

use futures::executor::block_on;
use wgpu::{util::DeviceExt, *};
use winit::{dpi, event::*, event_loop::ControlFlow, window::Window};

use bytemuck_derive::*;
use chrono::Timelike;
// use egui::app::App;
// use egui::paint::FontDefinitions;
use egui_wgpu_backend::{RenderPass, ScreenDescriptor};
use egui_winit_platform::{Platform, PlatformDescriptor};
use legion::*;
use rayon::{ThreadPool, ThreadPoolBuilder};
use std::borrow::Cow;

use anyhow::{anyhow, Context, Result};

use vk_shader_macros::include_glsl;

static VERTEX_SHADER: &[u32] = include_glsl!("src/shaders/test.vert");
static FRAGMENT_SHADER: &[u32] = include_glsl!("src/shaders/test.frag");

use argh::FromArgs;

#[repr(C)]
#[derive(Copy, Clone, Debug, Zeroable, Pod)]
struct Vertex {
    position: [f32; 3],
    color: [f32; 3],
}

#[derive(FromArgs)]
/// Grid Locked, the Game, finally becoming a reality this time I swear.
struct Arguments {
    /// the number of processing threads used to drive the engine.
    /// When unspecified or set to 0, will automatically determine the ideal
    /// number of threads to use on your system.
    #[argh(option, default = "0")]
    num_threads: usize,
}

// use crate::ecs::*;
// use crate::gui;

pub struct Client {
    // General graphics stuff.
    window: Window,
    surface: Surface,
    device: Device,
    queue: Queue,
    sc_desc: SwapChainDescriptor,
    swap_chain: SwapChain,
    size: dpi::PhysicalSize<u32>,
    render_pipeline: wgpu::RenderPipeline, // TODO should that go into a vector of some sort?
    vertex_buffer: wgpu::Buffer,           // TODO this should definitely not be here, but it's here for the experiments.

    // Egui stuff.
    platform: Platform,
    egui_rpass: RenderPass,

    // World simulation stuff.
    thread_pool: ThreadPool,
    worlds: Vec<(World, Schedule, Resources, legion::systems::CommandBuffer)>,
}

impl Client {
    async fn request_device(adapter: &Adapter) -> Result<(Device, Queue), RequestDeviceError> {
        adapter
            .request_device(
                &DeviceDescriptor { features: Features::empty(), limits: Limits::default(), label: None },
                None, // Trace path
            )
            .await
    }

    async fn request_adapter(instance: &Instance, surface: &Surface) -> Option<Adapter> {
        instance
            .request_adapter(&RequestAdapterOptions {
                power_preference: PowerPreference::HighPerformance, // TODO make this an option.
                compatible_surface: Some(surface),
            })
            .await
    }

    pub fn create_with_window(window: Window) -> Result<Client> {
        let size = window.inner_size();

        // The instance is a handle to the graphics driver.
        // BackendBit::PRIMARY => Vulkan + Metal + DX12 + Browser WebGPU
        let instance = Instance::new(BackendBit::PRIMARY);

        // Is unsafe because it depends on the window returning a valid descriptor.
        let surface = unsafe { instance.create_surface(&window) };

        // Grab the graphics adapter (the GPU outputting to the display)
        let adapter =
            block_on(Self::request_adapter(&instance, &surface)).ok_or(anyhow!("Failed to find graphics adapter."))?;

        // Get the actual GPU now.
        let (device, mut queue) = block_on(Self::request_device(&adapter))?;

        // Swap chain basically manages our double buffer.
        let sc_desc = SwapChainDescriptor {
            usage: TextureUsage::OUTPUT_ATTACHMENT,
            format: TextureFormat::Bgra8UnormSrgb,
            width: size.width,
            height: size.height,
            present_mode: PresentMode::Mailbox, // TODO let the user pick
        };
        let swap_chain = device.create_swap_chain(&surface, &sc_desc);

        let render_pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Render Pipeline Layout"),
            bind_group_layouts: &[],
            push_constant_ranges: &[],
        });

        let vs_module = device.create_shader_module(ShaderModuleSource::SpirV(Cow::Borrowed(VERTEX_SHADER)));
        let fs_module = device.create_shader_module(ShaderModuleSource::SpirV(Cow::Borrowed(FRAGMENT_SHADER)));

        let render_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Render Pipeline"),
            layout: Some(&render_pipeline_layout),
            vertex_stage: wgpu::ProgrammableStageDescriptor { module: &vs_module, entry_point: "main" },
            fragment_stage: Some(wgpu::ProgrammableStageDescriptor { module: &fs_module, entry_point: "main" }),
            rasterization_state: Some(wgpu::RasterizationStateDescriptor {
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: wgpu::CullMode::Back,
                depth_bias: 0,
                depth_bias_slope_scale: 0.0,
                depth_bias_clamp: 0.0,
                clamp_depth: false,
            }),
            color_states: &[wgpu::ColorStateDescriptor {
                format: sc_desc.format,
                color_blend: wgpu::BlendDescriptor::REPLACE,
                alpha_blend: wgpu::BlendDescriptor::REPLACE,
                write_mask: wgpu::ColorWrite::ALL,
            }],
            primitive_topology: wgpu::PrimitiveTopology::TriangleList,
            depth_stencil_state: None,
            vertex_state: wgpu::VertexStateDescriptor {
                index_format: wgpu::IndexFormat::Uint16,
                vertex_buffers: &[Vertex::desc()],
            },
            sample_count: 1,
            sample_mask: !0,
            alpha_to_coverage_enabled: false,
        });

        let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Vertex Buffer"),
            contents: bytemuck::cast_slice(VERTICES),
            usage: wgpu::BufferUsage::VERTEX,
        });

        // Grab arguments provided from the command line.
        let arguments: Arguments = argh::from_env();
        let thread_pool = ThreadPoolBuilder::new().num_threads(arguments.num_threads).build()?;

        let worlds = Vec::new();

        // We use the egui_winit_platform crate as the platform.
        let platform = Platform::new(PlatformDescriptor {
            physical_width: size.width as u32,
            physical_height: size.height as u32,
            scale_factor: window.scale_factor(),
            font_definitions: FontDefinitions::with_pixels_per_point(window.scale_factor() as f32),
            style: Default::default(),
        });

        let egui_rpass = RenderPass::new(&device, TextureFormat::Bgra8UnormSrgb);
        let demo_app = egui::demos::DemoApp::default();
        let demo_env = egui::demos::DemoEnvironment::default();

        Ok(Client {
            window,
            surface,
            device,
            queue,
            sc_desc,
            swap_chain,
            size,
            render_pipeline,
            vertex_buffer,
            platform,
            egui_rpass,
            demo_app,
            demo_env,
            worlds,
            thread_pool,
        })
    }

    pub fn process_event<T>(&mut self, event: &winit::event::Event<T>) -> Option<ControlFlow> {
        self.platform.handle_event(event);
        // TODO update time.

        let control_flow = match event {
            Event::WindowEvent { ref event, window_id } if *window_id == self.window.id() => match event {
                WindowEvent::CloseRequested => Some(ControlFlow::Exit),
                WindowEvent::KeyboardInput { input, .. } => match input {
                    KeyboardInput { state: ElementState::Pressed, virtual_keycode: Some(VirtualKeyCode::Escape), .. } => {
                        Some(ControlFlow::Exit)
                    }
                    _ => None,
                },
                WindowEvent::Resized(new_size) => {
                    self.on_resize(*new_size);
                    None
                }
                _ => None,
            },
            Event::RedrawRequested(_) => {
                let time = chrono::Local::now().time();
                let time_delta = time.num_seconds_from_midnight() as f64 + 1e-9 * (time.nanosecond() as f64);
                self.platform.update_time(time_delta);

                self.on_frame();
                None
            }
            Event::MainEventsCleared => {
                // RedrawRequested will only trigger once, unless we manually
                // request it.
                self.window.request_redraw();
                None
            }

            _ => None,
        };

        control_flow
    }

    fn on_resize(&mut self, new_size: dpi::PhysicalSize<u32>) {
        self.size = new_size;
        self.sc_desc.width = new_size.width;
        self.sc_desc.height = new_size.height;
        self.swap_chain = self.device.create_swap_chain(&self.surface, &self.sc_desc);
    }

    fn on_frame(&mut self) {
        for (world, schedule, resources, _command_buffer) in &mut self.worlds {
            schedule.execute_in_thread_pool(world, resources, &self.thread_pool);
        }

        let frame = self.swap_chain.get_current_frame();

        match frame {
            Ok(frame) => {
                let frame = frame.output;
                let mut encoder =
                    self.device.create_command_encoder(&wgpu::CommandEncoderDescriptor { label: Some("encoder") });

                // Render UI

                // TODO most of this could be done in another thread, or in parallel.
                let mut ui = self.platform.begin_frame();

                let mut integration_context = egui::app::IntegrationContext {
                    info: egui::app::IntegrationInfo {
                        web_info: None,
                        cpu_usage: None,
                        seconds_since_midnight: None,
                        native_pixels_per_point: None,
                    },
                    tex_allocator: Some(&mut self.egui_rpass),
                    output: Default::default(),
                };
                self.demo_app.ui(&self.platform.context(), &mut integration_context);

                let (_output, paint_commands) = self.platform.end_frame();
                let paint_jobs = self.platform.context().tesselate(paint_commands);

                let screen_descriptor = ScreenDescriptor {
                    physical_width: self.sc_desc.width,
                    physical_height: self.sc_desc.height,
                    scale_factor: self.window.scale_factor() as f32,
                };

                self.egui_rpass.update_texture(&self.device, &self.queue, &self.platform.context().texture());
                self.egui_rpass.update_buffers(&mut self.device, &mut self.queue, &paint_jobs, &screen_descriptor);

                self.egui_rpass.execute(&mut encoder, &frame.view, &paint_jobs, &screen_descriptor, Some(wgpu::Color::BLACK));

                // Render World.
                {
                    let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                        color_attachments: &[wgpu::RenderPassColorAttachmentDescriptor {
                            attachment: &frame.view,
                            resolve_target: None,
                            ops: wgpu::Operations { load: wgpu::LoadOp::Load, store: true },
                        }],
                        depth_stencil_attachment: None,
                    });
                    render_pass.set_pipeline(&self.render_pipeline);
                    render_pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
                    render_pass.draw(0..VERTICES.len() as u32, 0..1);
                }

                self.queue.submit(std::iter::once(encoder.finish()));
            }
            Err(error) => {
                log::error!("Error getting render frame: {}", error);
            }
        }
    }
}
