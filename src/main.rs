extern crate sdl2; 
extern crate num;

use std::vec::Vec;

use num::complex;

use sdl2::pixels::Color;
use sdl2::event::Event;
use sdl2::keyboard::Keycode;
use std::time::Duration;
use sdl2::render::Canvas;
use sdl2::video::Window;
use sdl2::gfx::primitives::DrawRenderer;

const DIMX: i32 = 8;
const DIMY: i32 = 6;

type Complex = complex::Complex<i32>;
const I : Complex = Complex::new(0, 1);

// main directions
const DIRECTIONS: [Complex; 4] = [
    Complex::new(1, 0),
    Complex::new(0, 1),
    Complex::new(-1, 0),
    Complex::new(0, -1),
];

const PLAYER_COUNT: usize = 2;
const PLAYER_COLORS: [Color; PLAYER_COUNT] = [
    Color::RGB(200, 0, 0),
    Color::RGB(0, 200, 0),
];

#[derive(Clone)]
#[derive(Copy)]
struct Marble {
    pos: Complex,
    target: Complex,
    owner: usize,
}
impl Marble {
    fn new(pos: Complex, owner: usize) -> Marble {
        Marble {
            pos: pos,
            target: pos,
            owner: owner,
        }
    }

    fn step(&mut self, remaining_steps: i32) {
        self.pos = self.target
            + ((self.pos - self.target) * remaining_steps) / (remaining_steps + 1);
    }

    fn draw(&self, canvas: &mut Canvas<Window>) -> Result<(), String> {
        let x = self.pos.re as i16;
        let y = self.pos.im as i16;
        let color = PLAYER_COLORS[self.owner];
        canvas.aa_circle(x, y, 15, color)?;
        canvas.filled_circle(x, y, 15, color)?;
        Ok(())
    }
}

struct Cell {
    owner: Option<usize>,
    coord: Complex,
    has_neighbor: [bool; 4],
    // Marbles that are held by this cell
    marbles: [Option<Marble>; 4],
    // Marbles that are on their way here.
    receiving: [Option<Marble>; 4],
}

impl Cell {
    fn new(coord: Complex) -> Cell {
        Cell {
            owner: None,
            coord: coord,
            has_neighbor: [
                coord.re < DIMX - 1,
                coord.im > 0,
                coord.re > 0,
                coord.im < DIMY - 1,
            ],
            marbles: [None, None, None, None],
            receiving: [None, None, None, None],
        }
    }

    fn add(&mut self, owner: usize) -> Result<(), ()>{
        if *self.owner.get_or_insert(owner) != owner {
            // Set owner if it is not yet set, but return an error if it is set differently
            return Err(())
        }
        for direction in 0..4 {
            if !self.has_neighbor[direction] {
                continue
            }
            if self.marbles[direction].is_none() {
                let coord = self.coord * 100 + Complex::new(50, 50) + 25*DIRECTIONS[direction];
                self.marbles[direction].insert(Marble::new(coord, owner));
                break;
            }
        }
        Ok(())
    }

    fn draw(&self, canvas: &mut Canvas<Window>) -> Result<(), String> {
        let center = self.coord * 100 + Complex::new(50, 50);
        for direction in 0..4 {
            if !self.has_neighbor[direction] {
                continue
            }
            let pos = center + 25*DIRECTIONS[direction];
            let x = pos.re as i16;
            let y = pos.im as i16;
            canvas.filled_circle(x, y, 15, Color::RGB(255, 255, 255))?;
            canvas.aa_circle(x, y, 15, Color::RGB(0, 0, 0))?;
        }
        for marble in self.marbles.into_iter().flatten() {
            marble.draw(canvas)?;
        }
        Ok(())
    }
}

enum State {
    AcceptingInput,
    Animating,
}

struct Grid {
    cells: Vec<Cell>,
    state: State,
    active_player: usize,
}

impl Grid {
    fn cell(&self, coordinates: Complex) -> &Cell {
        &self.cells[(DIMY*coordinates.re + coordinates.im) as usize]
    }
    fn cell_mut(&mut self, coordinates: Complex) -> &mut Cell {
        &mut self.cells[(DIMY*coordinates.re + coordinates.im) as usize]
    }
    fn new() -> Grid {
        let mut cells = Vec::with_capacity((DIMX*DIMY) as usize);
        for coordx in 0..DIMX {
            for coordy in 0..DIMY {
                cells.push(Cell::new(coordx+I*coordy))
            }
        }
        Grid{
            cells: cells,
            state: State::AcceptingInput,
            active_player: 0,
        }
    }

    fn click(&mut self, x: i32, y: i32) {
        match self.state {
            State::AcceptingInput => { },
            _ => return
        }

        let x = x/100;
        let y = y/100;
        if x >= DIMX || y >= DIMY {
            return
        }
        let active_player = self.active_player;
        match self.cell_mut(x+y*I).add(active_player) {
            Ok(_) => {
                self.active_player = (self.active_player + 1) % PLAYER_COUNT;
            },
            Err(_) => {}
        }
    }
    
    fn draw(&self, canvas: &mut Canvas<Window>) -> Result<(), String> {
        canvas.set_draw_color(Color::RGB(200, 200, 200));
        canvas.clear();
        for i in 0..DIMX + 1 {
            canvas.vline((i*100) as i16, 0, 100*DIMY as i16, Color::RGB(0, 0, 0))?;
        }
        for i in 0..DIMY {
            canvas.hline(0, (100*DIMX) as i16, (i*100) as i16, Color::RGB(0, 0, 0))?;
        }
        for player in 0..PLAYER_COUNT {
            let x = (DIMX * 100 + 50) as i16;
            let y = (30 + player as i16 * 40) as i16;
            canvas.filled_circle(x, y, 15, PLAYER_COLORS[player])?;
            canvas.aa_circle(x, y, 15, Color::RGB(0, 0, 0))?;
            if player == self.active_player {
                canvas.filled_pie(x-20, y, 20, 160, 200, Color::RGB(0, 0, 0))?;
            }
        }
        for cell in self.cells.iter() {
            cell.draw(canvas)?;
        }
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
                    grid.click(x, y);
                },
                _ => {}
            }
        }
        grid.draw(&mut canvas)?;
        canvas.present();
        std::thread::sleep(Duration::new(0, 1_000_000_000u32 / 60));
    };
    Ok(())
}
