#![allow(dead_code)]

mod app;
mod vulkan;

fn main() -> () {
    let app = app::run();

    match app {
        Ok(_) => println!("[APP] : SUCCESS"),
        Err(e) => println!("[APP] : ERROR = {}", e),
    };
}
