// Copyright James Carl (C) 2020
// AGPL-3.0-or-later

use futures::executor::block_on;
use wgpu::*;
use winit::{dpi, event::WindowEvent, window::Window};

use legion::{Resources, Schedule, World};
use rayon::{ThreadPool, ThreadPoolBuilder};

use num_traits::cast::FromPrimitive;

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

pub struct Client {
    surface: Surface,
    device: Device,
    queue: Queue,
    sc_desc: SwapChainDescriptor,
    swap_chain: SwapChain,
    size: dpi::PhysicalSize<u32>,
    thread_pool: ThreadPool,
    worlds: Vec<(World, Schedule, Resources)>,
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

    pub fn create_with_window(window: &Window) -> Result<Client, Box<dyn std::error::Error>> {
        let size = window.inner_size();

        // The instance is a handle to our GPU
        // BackendBit::PRIMARY => Vulkan + Metal + DX12 + Browser WebGPU
        let instance = Instance::new(BackendBit::PRIMARY);
        let surface = unsafe { instance.create_surface(window) };

        let adapter = block_on(Self::request_adapter(&instance, &surface)).unwrap();
        let (device, queue) = block_on(Self::request_device(&adapter)).unwrap();

        let sc_desc = SwapChainDescriptor {
            usage: TextureUsage::OUTPUT_ATTACHMENT,
            format: TextureFormat::Bgra8UnormSrgb,
            width: size.width,
            height: size.height,
            present_mode: PresentMode::Fifo,
        };
        let swap_chain = device.create_swap_chain(&surface, &sc_desc);

        let arguments: Arguments = argh::from_env();
        let thread_pool = ThreadPoolBuilder::new().num_threads(arguments.num_threads).build()?;

        use common::physics::{self, *};

        let mut world = World::default();
        let mut resources = Resources::default();
        let mut schedule_builder = Schedule::builder();
        physics::add_systems(&mut schedule_builder);
        let mut schedule = schedule_builder.build();

        world.push((
            Positional::new(PhysicsVec3::center_bottom_of_block(0, 0, 0).unwrap(), PhysicsScalar::from_i64(0).unwrap()),
            Movable::new(PhysicsScalar::from_i64(0).unwrap(), PhysicsVec3::zeroed(), PhysicsScalar::from_i64(0).unwrap()),
            CylinderPhysicalForm::new(PhysicsScalar::from_f32(0.4).unwrap(), PhysicsScalar::from_i64(2).unwrap()),
        ));

        let mut worlds = Vec::new();
        worlds.push((world, schedule, resources));

        Ok(Client { surface, device, queue, sc_desc, swap_chain, size, worlds, thread_pool })
    }

    pub fn on_resize(&mut self, new_size: winit::dpi::PhysicalSize<u32>) {
        self.size = new_size;
        self.sc_desc.width = new_size.width;
        self.sc_desc.height = new_size.height;
        self.swap_chain = self.device.create_swap_chain(&self.surface, &self.sc_desc);
    }

    pub fn process_event(&mut self, event: &WindowEvent) {
        unimplemented!()
    }

    pub fn update(&mut self) {
        for (world, schedule, resources) in &mut self.worlds {
            schedule.execute_in_thread_pool(world, resources, &self.thread_pool);
        }
    }

    pub fn render(&mut self) {
        let frame = self.swap_chain.get_current_frame().expect("Timeout getting texture").output;

        let mut encoder = self.device.create_command_encoder(&CommandEncoderDescriptor { label: Some("Render Encoder") });

        {
            let _render_pass = encoder.begin_render_pass(&RenderPassDescriptor {
                color_attachments: &[RenderPassColorAttachmentDescriptor {
                    attachment: &frame.view,
                    resolve_target: None,
                    ops: Operations { load: LoadOp::Clear(Color { r: 0.1, g: 0.2, b: 0.3, a: 1.0 }), store: true },
                }],
                depth_stencil_attachment: None,
            });
        }

        // submit will accept anything that implements IntoIter
        self.queue.submit(std::iter::once(encoder.finish()));
    }
}
