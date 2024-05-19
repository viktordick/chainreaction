use std::vec::Vec;
use std::time::Duration;

use sdl2::EventPump;
use sdl2::VideoSubsystem;
use sdl2::event::Event;
use sdl2::keyboard::Keycode;
use sdl2::pixels::Color;
use sdl2::rect::Rect;
use sdl2::gfx::primitives::DrawRenderer;

use crate::grid::Point;
use crate::game::Player;
use crate::render::{create_texture, gradient};

fn color(x: u8, y: u8) -> Color {
    // Map a 256x256 square onto a color, separating into six segments with the primary and
    // secondary colors at the edges and black in the center:
    // r - rg - g
    // |        |
    // rb - b - gb
    if x < 128 {
        if y <= x {
            Color::RGB(255-2*y, 2*(x-y), 0)
        }
        else if y >= 255 - x {
            Color::RGB(255-2*x, 0, 2*(y-128))
        }
        else {
            Color::RGB(255-2*x, 0, y-x)
        }
    }
    else {
        if y <= 255 - x {
            Color::RGB(2*(255-x-y), 255-2*y, 0)
        }
        else if y >= x {
            Color::RGB(0, 2*(x-128), 2*(y-128))
        }
        else {
            Color::RGB(0, 2*(x-128), y-(255-x))
        }
    }
}

pub struct Config {
    pub players: Vec<Player>,
    pub size: Point,
}

pub fn show_menu(video: &VideoSubsystem, event_pump: &mut EventPump) -> Result<Config, String> {
    let mut canvas = video
        .window("Chain reaction", 1024, 512)
        .position_centered()
        .resizable()
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
    let mut size = Point::new(8, 6);
    let mut marbles = Vec::new();
    let mut mousepos = (256,256);
    let mut next_color: Color = Color::RGB(0,0,0);
    'running: loop {
        for event in event_pump.poll_iter() {
            match event {
                Event::Quit {..}
                | Event::KeyDown { keycode: Some(Keycode::Escape | Keycode::Return), .. }
                => {
                    break 'running
                },
                Event::MouseMotion {x, y, ..} => {
                    mousepos = (x as i16, y as i16);
                    let p: (Result<u8, _>, Result<u8, _>) = ((x/2).try_into(), (y/2).try_into());
                    if let (Ok(x), Ok(y)) = p {
                        next_color = color(x, y);
                    }
                },
                Event::MouseButtonDown {.. } => {
                    if mousepos.0 < 512 {
                        players.push(Player::new(next_color));
                        marbles.push(
                            create_texture(&creator, 31, 31, |canvas| {
                                gradient(&canvas, 15, 15, next_color)?;
                                Ok(())
                            })?
                        );
                    } else if mousepos.0 > 525 && mousepos.1 > 55 {
                        size.re = ((mousepos.0 - 525)/10) as i32;
                        size.im = ((mousepos.1 - 55)/10) as i32;
                    }
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
        canvas.copy(&texture_bg, None, Some(Rect::new(0,0,512,512)))?;
        if mousepos.0 < 512 {
            canvas.filled_circle(mousepos.0, mousepos.1, 20, next_color)?;
        };
        for (i, marble) in marbles.iter().enumerate() {
            canvas.copy(&marble, None, Some(Rect::new(512+15 + i as i32 * 40, 15, 31, 31)))?;
        }
        let black = Color::RGB(0, 0, 0);
        for x in 0..=size.re as i16 {
            canvas.vline(525+10*x, 55, 55+10*size.im as i16, black)?;
        }
        for y in 0..=size.im as i16 {
            canvas.hline(525, 525+10*size.re as i16, 55+10*y, black)?;
        }
        canvas.present();
        std::thread::sleep(Duration::new(0, 1_000_000_000u32 / 60));
    };
    Ok(Config{
        players: players,
        size: size,
    })
}
