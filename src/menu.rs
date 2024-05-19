use std::vec::Vec;
use std::time::Duration;

use sdl2::EventPump;
use sdl2::VideoSubsystem;
use sdl2::event::{Event,WindowEvent};
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
    pub cellsize: i32,
}

pub fn show_menu(video: &VideoSubsystem, event_pump: &mut EventPump) -> Result<Config, String> {
    let mut canvas = video
        .window("Chain reaction", 0, 0)
        .fullscreen_desktop()
        .allow_highdpi()
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

    // In case of fractional scaling, this describes the "virtual" size in pixels, i.e. mouse
    // events are relative to this.
    let mut window_size = (0, 0);
    let mut players = Vec::new();
    let mut size = Point::new(8, 6);
    let mut marbles = Vec::new();
    let mut mousepos = (0u32, 0u32);
    let mut next_color: Option<Color> = None;
    'running: loop {
        // Actual number of pixels
        let output_size = canvas.output_size()?;
        for event in event_pump.poll_iter() {
            match event {
                Event::KeyDown { keycode: Some(Keycode::Escape | Keycode::Return), .. }
                | Event::Quit {..} => {
                    break 'running
                },
                Event::Window { win_event: WindowEvent::Resized(w, h), .. } => {
                    window_size = (w, h);
                },
                Event::MouseMotion {x, y, ..} => {
                    if window_size.0 > 0 {
                        mousepos = (
                            (x as f32 / window_size.0 as f32 * output_size.0 as f32) as u32,
                            (y as f32 / window_size.1 as f32 * output_size.1 as f32) as u32,
                        );
                        let offset = (output_size.1 as u32-512)/2;
                        if mousepos.0 >= 50 && mousepos.0 < 562
                            && mousepos.1 >= offset && mousepos.1 < offset + 512 {
                            next_color = Some(color(
                                ((mousepos.0-50)/2) as u8,
                                ((mousepos.1-offset)/2) as u8,
                            ));
                        } else {
                            next_color = None;
                        }
                    }
                },
                Event::MouseButtonDown { .. } => {
                    if let Some(col) = next_color {
                        players.push(Player::new(col));
                        marbles.push(
                            create_texture(&creator, 61, 61, |canvas| {
                                gradient(&canvas, 30, 30, 30, col)?;
                                Ok(())
                            })?
                        );
                    }
                    if mousepos.0 > 600 && mousepos.1 > 320 {
                        size.re = ((mousepos.0 - 600)/50) as i32;
                        size.im = ((mousepos.1 - 320)/50) as i32;
                        if size.re > 9 {
                            size.re = 9;
                        }
                        if size.im > 9 {
                            size.im = 9;
                        }
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
        if output_size.0 > 600 && output_size.1 > 600 {
            canvas.copy(
                &texture_bg, None,
                Some(Rect::new(50, (output_size.1 as i32-512)/2,512,512))
            )?;
        }
        if let Some(col) = next_color {
            canvas.filled_circle(mousepos.0 as i16, mousepos.1 as i16, 20, col)?;
        };
        for (i, marble) in marbles.iter().enumerate() {
            canvas.copy(&marble, None, Some(Rect::new(600 + i as i32 * 70, 50, 61, 61)))?;
        }
        let black = Color::RGB(0, 0, 0);
        for x in 0..=size.re as i16 {
            canvas.vline(600+50*x, 320, 320+50*size.im as i16, black)?;
        }
        for y in 0..=size.im as i16 {
            canvas.hline(600, 600+50*size.re as i16, 320+50*y, black)?;
        }
        canvas.present();
        std::thread::sleep(Duration::new(0, 1_000_000_000u32 / 60));
    };
    Ok(Config{
        players: players,
        size: size,
        cellsize: 100,
    })
}
