mod graphics;
mod event;

use std::time::Instant;

#[tokio::main]
async fn main() {
    let (tx, rx) = tokio::sync::broadcast::channel(1024);
    let mut window = graphics::Window::new(tx);
    let mut ctx = graphics::Graphics::new(&window, rx).await;
    let mut n = 0;
    let started = Instant::now();
    while !window.tick() {
        ctx.draw().await;
        n += 1;
    }
    println!(
        "FPS: {}",
        1.0 / ((Instant::now() - started).as_secs_f32() / n as f32)
    );
}