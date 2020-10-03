#[cfg(not(target_arch = "wasm32"))]
compile_error!("You are using the wrong compiler target. See the readme for details on how to fix that.");

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }
}
