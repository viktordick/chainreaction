extern crate sdl2; 

use std::vec::Vec;

use sdl2::pixels::Color;
use sdl2::event::Event;
use sdl2::keyboard::Keycode;
use std::time::Duration;
use sdl2::render::Canvas;
use sdl2::video::Window;
use sdl2::gfx::primitives::DrawRenderer;

const DIMX: usize = 8;
const DIMY: usize = 6;
const COORDS: [[usize; 4]; 2] = [
    [75, 50, 25, 50],
    [50, 25, 50, 75],
];

const PLAYER_COUNT: usize = 2;
const PLAYER_COLORS: [Color; PLAYER_COUNT] = [
    Color::RGB(200, 0, 0),
    Color::RGB(0, 200, 0),
];

fn draw_bg(canvas: &mut Canvas<Window>) -> Result<(), String> {
    canvas.set_draw_color(Color::RGB(200, 200, 200));
    canvas.clear();
    for i in 0..DIMX + 1 {
        canvas.vline((i*100) as i16, 0, 100*DIMY as i16, Color::RGB(0, 0, 0))?;
    }
    for i in 0..DIMY {
        canvas.hline(0, (100*DIMX) as i16, (i*100) as i16, Color::RGB(0, 0, 0))?;
    }
    for i in 0..PLAYER_COUNT {
        let x = DIMX * 100 + 50;
        let y = 30 + i as i16 * 40;
        canvas.filled_circle(x as i16, y as i16, 15, PLAYER_COLORS[i])?;
        canvas.aa_circle(x as i16, y as i16, 15, Color::RGB(0, 0, 0))?;
    }
    Ok(())
}

#[derive(Clone)]
#[derive(Copy)]
struct Marble {
    pos: [usize; 2],
    target: [usize; 2],
    owner: usize,
}
impl Marble {
    fn new(pos: [usize; 2], owner: usize) -> Marble {
        Marble {
            pos: pos,
            target: pos,
            owner: owner,
        }
    }

    fn step(&mut self, remaining_steps: usize) {
        for i in 0..2 {
            self.pos[i] = self.target[i]
                + ((self.pos[i] - self.target[i]) * remaining_steps) / (remaining_steps + 1);
        }
    }

    fn draw(&self, canvas: &mut Canvas<Window>) -> Result<(), String> {
        let x = self.pos[0] as i16;
        let y = self.pos[1] as i16;
        let color = PLAYER_COLORS[self.owner];
        canvas.aa_circle(x, y, 15, color)?;
        canvas.filled_circle(x, y, 15, color)?;
        Ok(())
    }
}

struct Field {
    owner: Option<usize>,
    coord: [usize; 2],
    directions: [Option<Vec<Marble>>; 4],
}

impl Field {
    fn new(coord: [usize; 2]) -> Field {
        let mut field = Field {
            owner: None,
            coord: coord,
            directions: [None, None, None, None],
        };
        let has_neighbor = [
            coord[0] < DIMX - 1,
            coord[1] > 0,
            coord[0] > 0,
            coord[1] < DIMY - 1,
        ];
        for i in 0..4 {
            if has_neighbor[i] {
                field.directions[i] = Some(Vec::with_capacity(2));
            }
        };
        field
    }

    fn add(&mut self, owner: usize) {
        assert_eq!(owner, *self.owner.get_or_insert(owner));
        for i in 0..4 {
            match &mut self.directions[i] {
                None => continue,
                Some(marbles) => {
                    if !marbles.is_empty() {
                        continue
                    }
                    let x = self.coord[0] * 100 + COORDS[0][i];
                    let y = self.coord[1] * 100 + COORDS[1][i];
                    marbles.push(Marble::new([x, y], owner));
                    break;
                },
            }
        }
    }

    fn draw(&self, canvas: &mut Canvas<Window>) -> Result<(), String> {
        for k in 0..4 {
            match &self.directions[k] {
                None => continue,
                Some(marbles) => {
                    let x = (self.coord[0] * 100 + COORDS[0][k]) as i16;
                    let y = (self.coord[1] * 100 + COORDS[1][k]) as i16;
                    canvas.filled_circle(x, y, 15, Color::RGB(255, 255, 255))?;
                    canvas.aa_circle(x, y, 15, Color::RGB(0, 0, 0))?;
                    for marble in marbles.iter() {
                        marble.draw(canvas)?;
                    }
                },
            }
        }
        Ok(())
    }
}

struct Grid {
    fields: Vec<Vec<Field>>,
}

impl Grid {
    fn new() -> Grid {
        let mut grid = Grid {
            fields: Vec::with_capacity(DIMX)
        };
        for i in 0..DIMX {
            grid.fields.push(Vec::with_capacity(DIMY));
            for j in 0..DIMY {
                grid.fields[i].push(Field::new([i, j]))
            };
        };
        grid
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
 
    let mut event_pump = sdl_context.event_pump()?;
    let mut grid = Grid::new();
    'running: loop {
        canvas.set_draw_color(Color::RGB(90, 90, 90));
        canvas.clear();
        for event in event_pump.poll_iter() {
            match event {
                Event::Quit {..} |
                Event::KeyDown { keycode: Some(Keycode::Escape), .. } => {
                    break 'running
                },
                Event::MouseButtonDown {x, y, .. } => {
                    let x = (x / 100) as usize;
                    let y = (y / 100) as usize;
                    if x < DIMX && y < DIMY {
                        grid.fields[x][y].add(0);
                    }
                },
                _ => {}
            }
        }
        // The rest of the game loop goes here...
        draw_bg(&mut canvas)?;
        for field in grid.fields.iter().flatten() {
            field.draw(&mut canvas)?;
        }
        canvas.present();
        std::thread::sleep(Duration::new(0, 1_000_000_000u32 / 60));
    };
    Ok(())
}
