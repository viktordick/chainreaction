use std::vec::Vec;
use std::time::Duration;

use sdl2::EventPump;
use sdl2::VideoSubsystem;
use sdl2::event::Event;
use sdl2::keyboard::Keycode;
use sdl2::pixels::Color;
use sdl2::rect::Rect;
use sdl2::gfx::primitives::DrawRenderer;

use crate::game::Player;
use crate::render::{create_texture, gradient};

fn color(x: u8, y: u8) -> Color {
    // Map a 256x256 square onto a color
    let sum = x as i16 + y as i16 - 256;
    Color::RGB(
        if x > 128 { 0 } else { 128 - y },
        if x > 128 { 0 } else { 128 - x },
        if sum < 0 { 0 } else { sum as u8 },
    )
}

pub fn show_menu(video: &VideoSubsystem, event_pump: &mut EventPump) -> Result<Vec<Player>, String> {
    let mut canvas = video
        .window("Chain reaction", 612, 512)
        .position_centered()
        .build()
        .map_err(|e| e.to_string())?
        .into_canvas()
        .present_vsync()
        .accelerated()
        .build()
        .map_err(|e| e.to_string())?;

    let creator = canvas.texture_creator();
    let texture_bg = create_texture(&creator, 256, 256, |canvas| {
        for x in 0..256 {
            for y in 0..256 {
                canvas.pixel(x, y, color(x as u8, y as u8))?;
            }
        }
        Ok(())
    })?;

    let mut players = Vec::new();
    let mut marbles = Vec::new();
    let mut next_color: Color = Color::RGB(255, 255, 255);
    'running: loop {
        for event in event_pump.poll_iter() {
            match event {
                Event::Quit {..}
                | Event::KeyDown { keycode: Some(Keycode::Escape | Keycode::Return), .. }
                => {
                    break 'running
                },
                Event::MouseMotion {x, y, ..} => {
                    let p: (Result<u8, _>, Result<u8, _>) = ((x/2).try_into(), (y/2).try_into());
                    if let (Ok(x), Ok(y)) = p {
                        next_color = color(x, y);
                    }
                },
                Event::MouseButtonDown {.. } => {
                    players.push(Player::new(next_color));
                    marbles.push(
                        create_texture(&creator, 31, 31, |canvas| {
                            gradient(&canvas, 15, 15, next_color)?;
                            Ok(())
                        })?
                    );
                },
                Event::KeyDown { keycode: Some(Keycode::Backspace), .. } => {
                    players.pop();
                    marbles.pop();
                },
                _ => continue,
            }
        }
        canvas.set_draw_color(Color::RGB(200, 200, 200));
        canvas.clear();
        canvas.set_draw_color(next_color);
        canvas.fill_rect(Rect::new(522, 0, 80, 10))?;
        canvas.copy(&texture_bg, None, Some(Rect::new(0,0,512,512)))?;
        for (i, marble) in marbles.iter().enumerate() {
            canvas.copy(&marble, None, Some(Rect::new(512+35, 15 + i as i32*40, 31, 31)))?;
        }
        canvas.present();
        std::thread::sleep(Duration::new(0, 1_000_000_000u32 / 60));
    };
    Ok(players)
}
