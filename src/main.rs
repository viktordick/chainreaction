extern crate sdl2; 

use sdl2::pixels::Color;
use sdl2::event::Event;
use sdl2::keyboard::Keycode;
use std::time::Duration;
use sdl2::render::Canvas;
use sdl2::video::Window;
use sdl2::gfx::primitives::DrawRenderer;

const DIMX: i16 = 8;
const DIMY: i16 = 6;
const STEPS: i32 = 8;

const PLAYER_COUNT: usize = 2;
const PLAYER_COLORS: [Color; PLAYER_COUNT] = [
    Color::RGB(200, 0, 0),
    Color::RGB(0, 200, 0),
];

fn draw_bg(canvas: &mut Canvas<Window>) -> Result<(), String> {
    canvas.set_draw_color(Color::RGB(200, 200, 200));
    canvas.clear();
    for i in 0..DIMX + 1 {
        canvas.vline(i*100, 0, 100*DIMY, Color::RGB(0, 0, 0))?;
    }
    for i in 0..DIMY {
        canvas.hline(0, 100*DIMX, i*100, Color::RGB(0, 0, 0))?;
    }
    for i in 0..DIMX {
        for j in 0..DIMY {
            for k in 0..4 {
                let x = [25, 50, 75, 50][k];
                let y = [50, 25, 50, 75][k];
                canvas.filled_circle(i*100+x, j*100+y, 15, Color::RGB(255, 255, 255))?;
                canvas.aa_circle(i*100 + x, j*100 + y, 15, Color::RGB(0, 0, 0))?;
            }
        }
    };
    for i in 0..PLAYER_COUNT {
        let x = DIMX * 100 + 50;
        let y = 30 + i as i16 * 40;
        canvas.filled_circle(x, y, 15, PLAYER_COLORS[i]);
        canvas.aa_circle(x, y, 15, PLAYER_COLORS[i]);
    }
    Ok(())
}

struct Marble {
    pos: [i32; 2],
    target: [i32; 2],
}
impl Marble {
    fn draw(&self, canvas: &mut Canvas<Window>, step: i32) -> Result<(), String> {
        let mut pos: [i32; 2] = [0, 0];
        for i in 0..2 {
            pos[i] = self.pos[i] + ((self.target[i] - self.pos[i])*step)/STEPS;
        }
        canvas.aa_circle(pos[0] as i16, pos[1] as i16, 15, Color::RGB(200, 0, 0))?;
        canvas.filled_circle(pos[0] as i16, pos[1] as i16, 15, Color::RGB(200, 0, 0))?;
        Ok(())
    }
}
 
pub fn main() -> Result<(), String> {
    let sdl_context = sdl2::init()?;
    let video_subsystem = sdl_context.video()?;
 
    let window = video_subsystem.window("Chain reaction", 100*DIMX as u32 + 100, 100*DIMY as u32)
        .position_centered()
        .build()
        .map_err(|e| e.to_string())?;
 
    let mut canvas = window
        .into_canvas()
        .present_vsync()
        .accelerated()
        .build()
        .map_err(|e| e.to_string())?;
 
    let mut marble = Marble{
        pos: [75, 50],
        target: [125, 50],
    };
    let mut event_pump = sdl_context.event_pump()?;
    let mut step: i32 = 0;
    'running: loop {
        canvas.set_draw_color(Color::RGB(90, 90, 90));
        canvas.clear();
        for event in event_pump.poll_iter() {
            match event {
                Event::Quit {..} |
                Event::KeyDown { keycode: Some(Keycode::Escape), .. } => {
                    break 'running
                },
                _ => {}
            }
        }
        // The rest of the game loop goes here...
        draw_bg(&mut canvas)?;
        step += 1;
        if step > STEPS {
            step = 0;
            let oldpos = marble.pos;
            marble.pos = marble.target;
            marble.target = oldpos;
        }
        marble.draw(&mut canvas, step);
        canvas.present();
        std::thread::sleep(Duration::new(0, 1_000_000_000u32 / 60));
    };
    Ok(())
}
