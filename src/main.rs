mod app;
mod bullet;
mod editor;
mod enemy;
mod game;
mod game_loop;
mod input;
mod level;
mod level_select;
mod menu;
mod movement;
mod player;
mod renderer;
mod spawn;
mod state;
mod text;
mod wall;

use winit::event_loop::{ControlFlow, EventLoop};

use app::App;

fn main() {
    let event_loop = EventLoop::new().unwrap();
    event_loop.set_control_flow(ControlFlow::Poll);
    let mut app = App::default();
    event_loop.run_app(&mut app).unwrap();
}
