// Copyright James Carl (C) 2020
// AGPL-3.0-or-later

use jemallocator::Jemalloc;

#[global_allocator]
static GLOBAL: Jemalloc = Jemalloc;

fn main() {
    println!("Hello, world!");
}
