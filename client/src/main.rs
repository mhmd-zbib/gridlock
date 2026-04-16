mod app;
mod net;
mod render;
mod systems;
mod ui;
mod util;

use winit::event_loop::{ControlFlow, EventLoop};

use app::App;

fn main() {
    let event_loop = EventLoop::new().unwrap();
    event_loop.set_control_flow(ControlFlow::Poll);
    let mut app = App::default();
    event_loop.run_app(&mut app).unwrap();
}
