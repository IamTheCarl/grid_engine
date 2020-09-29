//! Client for Grid Locked engine

#![warn(missing_docs)]

use jemallocator::Jemalloc;

// Use a global allocator that's better for threaded work.
#[global_allocator]
static GLOBAL: Jemalloc = Jemalloc;

use num_traits::cast::FromPrimitive;

// use vk_shader_macros::include_glsl;

// static VERTEX_SHADER: &[u32] = include_glsl!("shaders/test.vert");
// static FRAGMENT_SHADER: &[u32] = include_glsl!("shaders/test.frag");

// use winit::{
//     event::*,
//     event_loop::{EventLoop, ControlFlow},
//     window::{Window, WindowBuilder},
// };

fn main() {

    use specs::{WorldExt, Builder};

    let (mut world, dispatcher) = common::world::create_world();
    let mut dispatcher = dispatcher.build();
    dispatcher.setup(&mut world);

    use common::physics::*;

    world.
        create_entity()
        .with(Positional::new(
            PhysicsVec3::center_bottom_of_block(0, 0, 0).unwrap(),
            PhysicsScalar::from_i64(0).unwrap()))
        .with(Movable::new(PhysicsScalar::from_i64(0).unwrap(), PhysicsVec3::zeroed(), PhysicsScalar::from_i64(0).unwrap()))
        .with(CylinderPhysicalForm::new(PhysicsScalar::from_f32(0.4).unwrap(), PhysicsScalar::from_i64(2).unwrap()))
        .build();

    dispatcher.dispatch(&world);
    world.maintain();

    // env_logger::init();
    // let event_loop = EventLoop::new();
    // let window = WindowBuilder::new()
    //     .build(&event_loop)
    //     .unwrap(); // TODO no unwrap.

    // event_loop.run(move |event, _, control_flow| {
    //     match event {
    //         Event::WindowEvent {
    //             ref event,
    //             window_id,
    //         } if window_id == window.id() => match event {
    //             WindowEvent::CloseRequested => *control_flow = ControlFlow::Exit,
    //             WindowEvent::KeyboardInput {
    //                 input,
    //                 ..
    //             } => {
    //                 match input {
    //                     KeyboardInput {
    //                         state: ElementState::Pressed,
    //                         virtual_keycode: Some(VirtualKeyCode::Escape),
    //                         ..
    //                     } => *control_flow = ControlFlow::Exit,
    //                     _ => {}
    //                 }
    //             }
    //             _ => {}
    //         }
    //         _ => {}
    //     }
    // });
}
