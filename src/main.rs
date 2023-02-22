use std::time::Duration;

use sdl2::pixels::Color;
use sdl2::event::Event;
use sdl2::keyboard::Keycode;

mod game;
mod grid;
mod render;

use crate::grid::{DIM, Point};
use crate::game::Game;
use crate::render::Renderer;

 
pub fn main() -> Result<(), String> {
    let sdl_context = sdl2::init()?;
    let video_subsystem = sdl_context.video()?;
 
    let window = video_subsystem.window("Chain reaction", 100*DIM.re as u32 + 100, 100*DIM.im as u32)
        .position_centered()
        .build()
        .map_err(|e| e.to_string())?;

    let mut canvas = window
        .into_canvas()
        .present_vsync()
        .accelerated()
        .build()
        .map_err(|e| e.to_string())?;

    let mut game = Game::new();

    let texture_creator = canvas.texture_creator();
    let renderer = Renderer::new(&texture_creator, &game)?;

    let mut event_pump = sdl_context.event_pump()?;
    'running: loop {
        canvas.set_draw_color(Color::RGB(90, 90, 90));
        canvas.clear();
        for event in event_pump.poll_iter() {
            match event {
                Event::Quit {..} |
                Event::KeyDown { keycode: Some(Keycode::Escape), .. } => {
                    break 'running
                },
                Event::KeyDown { keycode, .. } => game.keydown(keycode.unwrap()),
                Event::MouseButtonDown {x, y, .. } => {
                    let x = x/100;
                    let y = y/100;
                    if x < DIM.re && y < DIM.im {
                        game.click(Point::new(x, y));
                    }
                },
                _ => {}
            }
        }
        game.step();
        renderer.update(&mut canvas, &game)?;
        canvas.present();
        std::thread::sleep(Duration::new(0, 1_000_000_000u32 / 60));
    };
    Ok(())
}
