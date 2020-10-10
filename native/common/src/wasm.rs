// Copyright James Carl (C) 2020
// AGPL-3.0-or-later

//! Management of web assembly assets.

// use legion::{
//     query::{
//         EntityFilter,
//         LayoutFilter,
//         DynamicFilter,
//         FilterResult,
//         GroupMatcher,
//         Fetch
//     },
//     storage::{
//         ComponentTypeId
//     },
//     world::{
//         WorldId
//     }
// };

// use std::default::Default;

// struct WasmComponent {
//     type_id: u32,
// }

// struct WasmFilter {
//     layout: WasmLayout,
//     dynamic: WasmDynamicFilter,
// }

// impl EntityFilter for WasmFilter {

//     type Layout = WasmLayout;
//     type Dynamic = WasmDynamicFilter;

//     fn layout_filter(&self) -> &Self::Layout {
//         &self.layout
//     }

//     fn filters(&mut self) -> (&Self::Layout, &mut Self::Dynamic) {
//         (&self.layout, &mut self.dynamic)
//     }
// }

// impl Default for WasmFilter {
//     fn default() -> Self {
//         unimplemented!()
//     }
// }

// struct WasmLayout;

// impl GroupMatcher for WasmLayout {
//     fn can_match_group() -> bool {
//         unimplemented!()
//     }

//     fn group_components() -> Vec<ComponentTypeId> {
//         unimplemented!()
//     }
// }

// impl LayoutFilter for WasmLayout {
//     fn matches_layout(&self, components: &[ComponentTypeId]) -> FilterResult {
//         // Check native type IDs
//         // Check wasm type IDs
//         unimplemented!()
//     }
// }

// impl Default for WasmLayout {
//     fn default() -> Self {
//         unimplemented!()
//     }
// }

// struct WasmDynamicFilter {
//     type_id: u32
// }

// impl DynamicFilter for WasmDynamicFilter {
//     fn prepare(&mut self, world: WorldId) {
//         unimplemented!()
//     }

//     fn matches_archetype<F: Fetch>(&mut self, fetch: &F) -> FilterResult {
//         if let Some(components) = fetch.find::<WasmComponent>() {
//             for component in components {
//                 if component.type_id == self.type_id {
//                     // TODO let a function internal to the WASM do this check against an actual Rust typeID.
//                     return FilterResult::Match(true);
//                 }
//             }

//             FilterResult::Match(false)
//         } else {
//             FilterResult::Defer
//         }
//     }
// }

// impl Default for WasmDynamicFilter {
//     fn default() -> Self {
//         WasmDynamicFilter {
//             type_id: 0
//         }
//     }
// }

// #[cfg(test)]
// mod testing {
//     use super::*;
//     use legion::query::Query;
//     use legion::SystemBuilder;

//     #[test]
//     fn wasm_query_build_filtering() {
//         let query: Query<WasmComponent, > = Query::new();

//         let system = SystemBuilder::new("TestSystem")
//             .with_query(query)
//             // .with_query(WasmFilter::default()::query())
//             .build(move |_commands, _world, _resource, _queries|{});
//     }
// }

// // For now I'm using the server as my sort of "scratch space" because it's so simple to experiment in right now.
// use common::modules::*;
// use std::path::PathBuf;
// use wasmtime::*;
// use std::io::prelude::*;

// let package = std::fs::File::open("../example_mod/target/example_mod.zip")?;
// let mut package = PackageFile::load(std::io::BufReader::new(package))?;

// let mut wasm_binary = Vec::new();

// {
//     // Get a reader for the file.
//     let mut wasm = package.get_wasm(&PathBuf::from("test_ui.wasm")).unwrap();

//     // Unpack it into memory.
//     wasm.read_to_end(&mut wasm_binary)?;
// }

// let store = Store::default();
// let module = Module::new(store.engine(), wasm_binary)?;
// let instance = Instance::new(&store, &module, &[])?;

// let add_one = instance
//     .get_func("add_one")
//     .ok_or(anyhow::format_err!("failed to find `add_one` function export"))?
//     .get1::<i32, i32>()?;

// log::info!("Got result: {}", add_one(5)?);
