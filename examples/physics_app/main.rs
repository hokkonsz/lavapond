mod app;
mod physics;

fn main() -> () {
    let app = app::run();

    match app {
        Ok(_) => println!("[APP] : SUCCESS"),
        Err(e) => println!("[APP] : ERROR = {}", e),
    };
}
