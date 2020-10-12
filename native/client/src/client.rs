// Copyright James Carl (C) 2020
// AGPL-3.0-or-later

use futures::executor::block_on;
use imgui::*;
use imgui_wgpu::Renderer;
use imgui_winit_support::WinitPlatform;
use wgpu::*;
use winit::{dpi, event::*, event_loop::ControlFlow, window::Window};

use legion::*;
use rayon::{ThreadPool, ThreadPoolBuilder};

use num_traits::cast::FromPrimitive;

use anyhow::{anyhow, Result, Context};

// use vk_shader_macros::include_glsl;

// static VERTEX_SHADER: &[u32] = include_glsl!("shaders/test.vert");
// static FRAGMENT_SHADER: &[u32] = include_glsl!("shaders/test.frag");

use argh::FromArgs;

#[derive(FromArgs)]
/// Grid Locked, the Game, finally becoming a reality this time I swear.
struct Arguments {
    /// the number of processing threads used to drive the engine.
    /// When unspecified or set to 0, will automatically determine the ideal number of threads to use on your system.
    #[argh(option, default = "0")]
    num_threads: usize,
}

use crate::ecs::*;
use crate::gui;

pub struct Client {
    // General graphics stuff.
    window: Window,
    surface: Surface,
    device: Device,
    queue: Queue,
    sc_desc: SwapChainDescriptor,
    swap_chain: SwapChain,
    size: dpi::PhysicalSize<u32>,

    // ImGui stuff.
    winit_platform: WinitPlatform,
    imgui_context: imgui::Context,
    imgui_renderer: Renderer,

    // World simulation stuff.
    thread_pool: ThreadPool,
    worlds: Vec<(World, Schedule, Resources, legion::systems::CommandBuffer)>,
}

impl Client {
    async fn request_device(adapter: &Adapter) -> Result<(Device, Queue), RequestDeviceError> {
        adapter
            .request_device(
                &DeviceDescriptor { features: Features::empty(), limits: Limits::default(), shader_validation: true },
                None, // Trace path
            )
            .await
    }

    async fn request_adapter(instance: &Instance, surface: &Surface) -> Option<Adapter> {
        instance
            .request_adapter(&RequestAdapterOptions {
                power_preference: PowerPreference::Default,
                compatible_surface: Some(surface),
            })
            .await
    }

    fn setup_imgui(window: &Window) -> Result<(WinitPlatform, imgui::Context)> {
        // Set up dear imgui
        let mut imgui = imgui::Context::create();
        let mut platform = WinitPlatform::init(&mut imgui);
        platform.attach_window(imgui.io_mut(), &window, imgui_winit_support::HiDpiMode::Default);
        imgui.set_ini_filename(None);

        let scale_factor = window.scale_factor();
        let font_size = (13.0 * scale_factor) as f32;
        imgui.io_mut().font_global_scale = (1.0 / scale_factor) as f32;

        imgui.fonts().add_font(&[FontSource::DefaultFontData {
            config: Some(imgui::FontConfig {
                oversample_h: 1,
                pixel_snap_h: true,
                size_pixels: font_size,
                ..Default::default()
            }),
        }]);

        Ok((platform, imgui))
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

        let (winit_platform, mut imgui_context) = Self::setup_imgui(&window)?;
        let imgui_renderer = Renderer::new(&mut imgui_context, &device, &mut queue, sc_desc.format);

        // Grab arguments provided from the command line.
        let arguments: Arguments = argh::from_env();
        let thread_pool = ThreadPoolBuilder::new().num_threads(arguments.num_threads).build()?;

        use common::physics::{self, *};

        // TODO dynamically load this world in via UI.
        let mut world = World::default();
        let command_buffer = legion::systems::CommandBuffer::new(&world);
        let resources = Resources::default();
        let mut schedule_builder = Schedule::builder();
        physics::add_systems(&mut schedule_builder);
        let schedule = schedule_builder.build();

        world.push((
            Positional::new(PhysicsVec3::center_bottom_of_block(0, 0, 0).unwrap(), PhysicsScalar::from_i64(0).unwrap()),
            Movable::new(PhysicsScalar::from_i64(0).unwrap(), PhysicsVec3::zeroed(), PhysicsScalar::from_i64(0).unwrap()),
            CylinderPhysicalForm::new(PhysicsScalar::from_f32(0.4).unwrap(), PhysicsScalar::from_i64(2).unwrap()),
            GUIComponent::new(gui::HelloWorld)
        ));

        // world.push();

        let mut worlds = Vec::new();
        worlds.push((world, schedule, resources, command_buffer));

        Ok(Client {
            window,
            surface,
            device,
            queue,
            sc_desc,
            swap_chain,
            size,
            winit_platform,
            imgui_context,
            imgui_renderer,
            worlds,
            thread_pool,
        })
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
                // RedrawRequested will only trigger once, unless we manually
                // request it.
                self.window.request_redraw();
                None
            }

            _ => None,
        };

        // Now let ImGUI handle events.
        self.winit_platform.handle_event(self.imgui_context.io_mut(), &self.window, event);

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

                let imgui = &mut self.imgui_context;
                let result = self.winit_platform.prepare_frame(imgui.io_mut(), &self.window);

                match result {
                    Ok(()) => {
                        let ui = imgui.frame();

                        // Since we can't prove to a system that our ui access is thread safe, we don't use a system and just directly
                        // call the gui components ourselves.
                        for (world, _schedule, _resources, command_buffer) in &mut self.worlds {
                            let mut query = <(Entity, &mut GUIComponent)>::query();
                            for (entity, gui) in query.iter_mut(world) {
                                let result = gui.on_frame(&ui)
                                    .context("Error while rendering GUI. Associated entity will be removed from world.");
                                if let Err(error) = result {
                                    log::error!("{:?}", error);
                                    command_buffer.remove(*entity);
                                }
                            }
                        }

                        let mut encoder =
                            self.device.create_command_encoder(&CommandEncoderDescriptor { label: Some("Render Encoder") });

                        {
                            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                                color_attachments: &[wgpu::RenderPassColorAttachmentDescriptor {
                                    attachment: &frame.view,
                                    resolve_target: None,
                                    ops: wgpu::Operations {
                                        load: wgpu::LoadOp::Clear(Color { r: 0.1, g: 0.2, b: 0.3, a: 1.0 }),
                                        store: true,
                                    },
                                }],
                                depth_stencil_attachment: None,
                            });

                            self.imgui_renderer
                                .render(ui.render(), &self.queue, &self.device, &mut render_pass)
                                .expect("Rendering failed");
                        }

                        // submit will accept anything that implements IntoIter
                        self.queue.submit(std::iter::once(encoder.finish()));
                    }
                    Err(error) => {
                        log::error!("Error getting ImGUI frame: {}", error);
                    }
                }
            }
            Err(error) => {
                log::error!("Error getting render frame: {}", error);
            }
        }
    }
}
