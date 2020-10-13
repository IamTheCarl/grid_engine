// Copyright James Carl (C) 2020
// AGPL-3.0-or-later

//! Client specific components and systems.

// use anyhow::Result;

// /// A trait for GUIs.
// pub trait GUI: std::fmt::Debug + Sync + Send {
//     /// Called every time the display is redrawn so that you can render the GUI to the display.
//     fn on_frame(&mut self, ui: &imgui::Ui) -> Result<()>;
// }

// /// A component that can contain a GUI, which will be rendered to the display.
// pub struct GUIComponent {
//     gui: Box<dyn GUI>,
// }

// impl GUIComponent {

//     /// Create a new GUIComponent containing a GUI.
//     pub fn new<T: 'static + GUI>(gui: T) -> GUIComponent {
//         GUIComponent {
//             gui: Box::new(gui)
//         }
//     }

//     /// Render the contained GUI to the display.
//     pub fn on_frame(&mut self, ui: &imgui::Ui) -> Result<()> {
//         self.gui.on_frame(ui)
//     }
// }