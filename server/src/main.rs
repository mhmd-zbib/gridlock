mod net;

use std::time::Duration;

use engine::input::InputState;
use game::game::Game;
use net::udp::run_udp_server;
use tokio::time::{Instant, sleep_until};

const TICK_RATE: u32 = 60;
const TICK_DURATION: Duration = Duration::from_nanos(1_000_000_000 / TICK_RATE as u64);
const UDP_PORT: u16 = 7777;

#[tokio::main]
async fn main() {
    println!("server starting (headless) — tick rate: {}Hz", TICK_RATE);
    println!("starting udp listener on port {}", UDP_PORT);

    let _udp_server = tokio::spawn(async move {
        if let Err(err) = run_udp_server(UDP_PORT).await {
            eprintln!("[udp] listener task exited: {err}");
        }
    });

    let mut game = Game::new();
    let input = InputState::default();

    let mut next_tick = Instant::now();
    let mut _tick: u64 = 0;

    loop {
        let now = Instant::now();

        if now >= next_tick {
            let dt = TICK_DURATION.as_secs_f32();
            game.update(dt, &input);
            _tick += 1;
            next_tick += TICK_DURATION;

            // If we've fallen behind by more than one tick, reset to avoid a spiral
            if Instant::now() > next_tick {
                next_tick = Instant::now() + TICK_DURATION;
            }
        } else {
            sleep_until(next_tick).await;
        }
    }
}
