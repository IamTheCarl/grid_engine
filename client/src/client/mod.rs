// Copyright James Carl (C) 2020-2021
// AGPL-3.0-or-later

use futures::executor::block_on;
use wgpu::util::DeviceExt;
use winit::{dpi, event::*, event_loop::ControlFlow, window::Window};

use bytemuck_derive::*;
use legion::*;

use anyhow::{anyhow, Result};

use graphics::GraphicsVector3;

const VERTICES: &[Vertex] = &[
    Vertex { position: GraphicsVector3::new(0.0, 0.5, 0.0), color: GraphicsVector3::new(1.0, 0.0, 0.0) },
    Vertex { position: GraphicsVector3::new(-0.5, -0.5, 0.0), color: GraphicsVector3::new(0.0, 1.0, 0.0) },
    Vertex { position: GraphicsVector3::new(0.5, -0.5, 0.0), color: GraphicsVector3::new(0.0, 0.0, 1.0) },
];

mod graphics;

use argh::FromArgs;

#[repr(C)]
#[derive(Copy, Clone, Debug, Zeroable, Pod)]
struct Vertex {
    position: nalgebra::Vector3<f32>,
    color: nalgebra::Vector3<f32>,
}

#[derive(FromArgs)]
/// Grid Locked, the Game, finally becoming a reality this time I swear.
struct Arguments {}

pub struct Client {
    // General graphics stuff.
    window: Window,
    surface: wgpu::Surface,
    device: wgpu::Device,
    queue: wgpu::Queue,
    sc_desc: wgpu::SwapChainDescriptor,
    swap_chain: wgpu::SwapChain,
    render_pipeline: wgpu::RenderPipeline, // TODO should that go into a vector of some sort?
    vertex_buffer: wgpu::Buffer,           // TODO this should definitely not be here, but it's here for the experiments.

    // World simulation stuff.
    worlds: Vec<(World, Schedule, Resources, legion::systems::CommandBuffer)>,
}

impl Client {
    async fn request_device(adapter: &wgpu::Adapter) -> Result<(wgpu::Device, wgpu::Queue), wgpu::RequestDeviceError> {
        adapter
            .request_device(
                &wgpu::DeviceDescriptor { features: wgpu::Features::empty(), limits: wgpu::Limits::default(), label: None },
                None, // Trace path
            )
            .await
    }

    async fn request_adapter(instance: &wgpu::Instance, surface: &wgpu::Surface) -> Option<wgpu::Adapter> {
        instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::HighPerformance, // TODO make this an option.
                compatible_surface: Some(surface),
            })
            .await
    }

    pub fn create_with_window(window: Window) -> Result<Client> {
        let size = window.inner_size();

        // The instance is a handle to the graphics driver.
        // BackendBit::PRIMARY => Vulkan + Metal + DX12 + Browser WebGPU
        let instance = wgpu::Instance::new(wgpu::BackendBit::PRIMARY);

        // Is unsafe because it depends on the window returning a valid descriptor.
        let surface = unsafe { instance.create_surface(&window) };

        // Grab the graphics adapter (the GPU outputting to the display)
        let adapter =
            block_on(Self::request_adapter(&instance, &surface)).ok_or(anyhow!("Failed to find graphics adapter."))?;

        // Get the actual GPU now.
        let (device, queue) = block_on(Self::request_device(&adapter))?;

        // Swap chain basically manages our double buffer.
        let sc_desc = wgpu::SwapChainDescriptor {
            usage: wgpu::TextureUsage::RENDER_ATTACHMENT,
            format: wgpu::TextureFormat::Bgra8UnormSrgb,
            width: size.width,
            height: size.height,
            present_mode: wgpu::PresentMode::Mailbox, // TODO let the user pick
        };
        let swap_chain = device.create_swap_chain(&surface, &sc_desc);
        let swap_chain_format =
            adapter.get_swap_chain_preferred_format(&surface).ok_or(anyhow!("Could not get swap chain's preferred format."))?;

        let render_pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Render Pipeline Layout"),
            bind_group_layouts: &[],
            push_constant_ranges: &[],
        });

        let shader_module = device.create_shader_module(&wgpu::include_spirv!(env!("gpu_code.spv")));

        let render_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Render Pipeline"),
            layout: Some(&render_pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader_module,
                entry_point: "main_vs",
                buffers: &[wgpu::VertexBufferLayout {
                    array_stride: std::mem::size_of::<Vertex>() as u64,
                    step_mode: wgpu::InputStepMode::Vertex,
                    attributes: &wgpu::vertex_attr_array!(0 => Float32x3, 1 => Float32x3),
                }],
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader_module,
                entry_point: "main_fs",
                targets: &[swap_chain_format.into()],
            }),
            primitive: wgpu::PrimitiveState::default(),
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
        });

        let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Vertex Buffer"),
            contents: bytemuck::cast_slice(VERTICES),
            usage: wgpu::BufferUsage::VERTEX,
        });

        // Grab arguments provided from the command line.
        let _arguments: Arguments = argh::from_env();

        let worlds = Vec::new();

        Ok(Client { window, surface, device, queue, sc_desc, swap_chain, render_pipeline, vertex_buffer, worlds })
    }

    pub fn process_event<T>(&mut self, event: &winit::event::Event<T>) -> Option<ControlFlow> {
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
                self.on_frame();
                None
            }
            Event::MainEventsCleared => {
                // RedrawRequested will only trigger once, unless we manually request it.
                self.window.request_redraw();
                None
            }

            _ => None,
        };

        control_flow
    }

    fn on_resize(&mut self, new_size: dpi::PhysicalSize<u32>) {
        self.sc_desc.width = new_size.width;
        self.sc_desc.height = new_size.height;
        self.swap_chain = self.device.create_swap_chain(&self.surface, &self.sc_desc);
    }

    fn on_frame(&mut self) {
        for (world, schedule, resources, _command_buffer) in &mut self.worlds {
            // Because parallel is enabled, this will use the global thread pool.
            schedule.execute(world, resources);
        }

        let frame = self.swap_chain.get_current_frame();

        match frame {
            Ok(frame) => {
                let frame = frame.output;
                let mut encoder =
                    self.device.create_command_encoder(&wgpu::CommandEncoderDescriptor { label: Some("encoder") });

                // Render World.
                {
                    let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                        label: None,
                        color_attachments: &[wgpu::RenderPassColorAttachment {
                            view: &frame.view,
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
