#![allow(dead_code)]

mod app2;
mod vulkan;

fn main() -> () {
    let app = app2::run();

    match app {
        Ok(_) => println!("[APP] : SUCCESS"),
        Err(e) => println!("[APP] : ERROR = {}", e),
    };
}

// TODO!
// Instance & Multiple Objects
// Set Colors
// TextBox
// Load Multiple Data
