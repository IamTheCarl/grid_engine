// Copyright James Carl (C) 2020
// AGPL-3.0-or-later

//! Implementations of GUIs.

// use anyhow::Result;
// use imgui::*;
// use crate::ecs;

// #[derive(Debug)]
// pub struct HelloWorld;

// impl ecs::GUI for HelloWorld {
//     fn on_frame(&mut self, ui: &Ui) -> Result<()> {
//         let window = imgui::Window::new(im_str!("Hello world"));
//         window.size([300.0, 100.0], Condition::FirstUseEver).build(&ui, || {
//             ui.text(im_str!("Hello world!"));
//             ui.text(im_str!("This...is...imgui-rs on WGPU!"));
//             ui.separator();
//             let mouse_pos = ui.io().mouse_pos;
//             ui.text(im_str!("Mouse Position: ({:.1},{:.1})", mouse_pos[0],
// mouse_pos[1]));         });

//         Ok(())
//     }
// }
