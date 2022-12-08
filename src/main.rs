extern crate sdl2; 
extern crate num;

use std::cmp::min;
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
const STEPS_TOTAL: i32 = 15;
const STEPS_PAUSE: i32 = 5;

type Complex = complex::Complex<i32>;
type Direction = usize;
type Owner = usize;

const I : Complex = Complex::new(0, 1);

// main directions
const DIRECTIONS: [Complex; 4] = [
    Complex::new(1, 0),
    Complex::new(0, 1),
    Complex::new(-1, 0),
    Complex::new(0, -1),
];

struct Player {
    started: bool,
    alive: bool,
    color: Color,
}
impl Player {
    fn new(red: u8, green: u8, blue: u8) -> Player {
        Player{
            started: false,
            alive: true,
            color: Color::RGB(red, green, blue),
        }
    }
}

#[derive(Clone)]
#[derive(Copy)]
struct Marble {
    // current position (exact position in pixels)
    pos: Complex,
    // target position we are moving to
    target: Complex,
    direction: Direction,
    owner: Owner,
}
impl Marble {
    fn new(pos: Complex, owner: Owner, direction: Direction) -> Marble {
        Marble {
            pos: pos,
            target: pos,
            owner: owner,
            direction: direction,
        }
    }

    fn step(&mut self, remaining_steps: i32) {
        if remaining_steps < STEPS_PAUSE {
            return;
        }
        self.pos = self.target
            + ((self.pos - self.target) * (remaining_steps-STEPS_PAUSE))
            / (remaining_steps - STEPS_PAUSE + 1);
    }

    fn draw(&self, canvas: &mut Canvas<Window>, color: Color) -> Result<(), String> {
        let x = self.pos.re as i16;
        let y = self.pos.im as i16;
        canvas.aa_circle(x, y, 15, color)?;
        canvas.filled_circle(x, y, 15, color)?;
        Ok(())
    }
}

struct Cell {
    owner: Option<usize>,
    coord: Complex,
    num_neighbors: usize,
    has_neighbor: [bool; 4],
    // For each direction, we have a vector of marbles. Some of them might be "transient",
    // i.e. not yet to be moved away in the current animation phase.
    // New marbles are added as non-transient.
    // At the start of the animation phase, we check if there are as many non-transient marbles as
    // there are neighbors - if so, we remove one from each direction and return it to the caller
    // (Grid), which adds them to the neighbors as transient marbles.
    // At the end of the animation phase, all marbles are marked as non-transient.
    marbles: Vec<Marble>,
    num_transients: usize,
}

impl Cell {
    fn new(coord: Complex) -> Cell {
        let has_neighbor = [
            coord.re < DIMX - 1,
            coord.im < DIMY-1,
            coord.re > 0,
            coord.im > 0,
        ];
        Cell {
            owner: None,
            coord: coord,
            has_neighbor: has_neighbor,
            num_neighbors: has_neighbor.iter().map(|x| if *x {1} else {0}).sum(),
            marbles: Vec::with_capacity(9),
            num_transients: 0,
        }
    }

    fn add(&mut self, owner: usize) -> Result<(), ()>{
        if *self.owner.get_or_insert(owner) != owner {
            // Set owner if it is not yet set, but return an error if it is set differently
            return Err(())
        }
        let mut free = self.has_neighbor.clone();
        for marble in self.marbles.iter() {
            free[marble.direction] = false;
        }
        for direction in 0..4 {
            if !free[direction] {
                continue;
            }
            let coord = self.coord * 100 + Complex::new(50, 50) + 25*DIRECTIONS[direction];
            self.marbles.push(Marble::new(coord, owner, direction));
            break;
        }
        Ok(())
    }

    fn check_overflow(&mut self) -> Vec<Marble> {
        // If all slots have marbles, push them into the result and remove them from the cell
        // itself.
        if self.marbles.len() - self.num_transients < self.num_neighbors {
            return vec![]
        }
        let mut result = Vec::with_capacity(4);
        let mut todo = self.has_neighbor.clone();
        let mut idx = 0;
        while idx < self.marbles.len() {
            let direction = self.marbles[idx].direction;
            if !todo[direction] {
                idx += 1;
                continue;
            }
            todo[direction] = false;
            result.push(self.marbles.remove(idx));
        }
        result
    }

    fn target_direction(&self, origin: Direction) -> Direction {
        // Strategy to find the direction where the marble should be facing:
        // a) Do not use a direction that already has more marbles than any other or one where no
        // neighbor exists
        // b) Prefer opposite of the original direction, i.e. from where it came, then those
        // neighboring that direction, then the original direction.
        let mut count = [0; 4];
        for existing in self.marbles.iter() {
            count[existing.direction] += 1;
        }
        let mincount = count.iter()
            .enumerate()
            .filter(|(idx, _)| self.has_neighbor[*idx])
            .fold(4, |acc, (_, x)| min(acc, *x));
        let mut is_candidate = self.has_neighbor.clone();
        for direction in 0..4 {
            is_candidate[direction] &= count[direction] == mincount
        }
        for rotation in [2, 1, 3, 0] {
            let direction = (origin+rotation)%4;
            if is_candidate[direction] {
                return direction;
            }
        }
        0 //should not happen
    }

    fn receive(&mut self, mut marble: Marble) {
        self.owner = Some(marble.owner);
        marble.direction = self.target_direction(marble.direction);
        marble.target = self.coord * 100 + Complex::new(50, 50) + 25*DIRECTIONS[marble.direction];
        self.marbles.push(marble);
        self.num_transients += 1;
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
    steps: i32,
    active_player: usize,
    players: Vec<Player>,
}

impl Grid {
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
        let mut players = vec![];
        players.push(Player::new(200, 0, 0));
        players.push(Player::new(0, 200, 0));
        players.push(Player::new(0, 0, 200));
        Grid{
            cells: cells,
            state: State::AcceptingInput,
            steps: 0,
            players: players,
            active_player: 0,
        }
    }

    fn spread(&mut self, coord: Complex) -> bool {
        let moving = self.cell_mut(coord).check_overflow();
        if moving.is_empty() {
            return false;
        }
        for marble in moving.into_iter() {
            self.cell_mut(coord + DIRECTIONS[marble.direction]).receive(marble)
        }
        true
    }

    fn next_player(&mut self) {
        loop {
            self.active_player = (self.active_player + 1)%self.players.len();
            if self.players[self.active_player].alive {
                return
            }
        }
    }

    fn click(&mut self, x: i32, y: i32) {
        match self.state {
            State::AcceptingInput => (),
            _ => return
        }

        let x = x/100;
        let y = y/100;
        if x >= DIMX || y >= DIMY {
            return
        }
        let active_player = self.active_player;
        self.players[active_player].started = true;
        let coord = x+y*I;
        let cell = self.cell_mut(coord);
        match cell.add(active_player) {
            Ok(_) => {
                if self.spread(coord) {
                    self.state = State::Animating;
                    self.steps = STEPS_TOTAL;
                } else {
                    self.next_player();
                }
            },
            Err(_) => {}
        }
    }

    fn step(&mut self) {
        match self.state {
            State::Animating => (),
            _ => return
        };

        for cell in self.cells.iter_mut() {
            for marble in cell.marbles.iter_mut() {
                marble.step(self.steps)
            }
        }
        if self.steps > 0 {
            self.steps -= 1;
            return;
        }
        for player in self.players.iter_mut() {
            player.alive = !player.started;
        }
        for cell in self.cells.iter_mut() {
            for marble in cell.marbles.iter_mut() {
                marble.owner = cell.owner.unwrap();
            }
            if cell.marbles.len() == 0 {
                cell.owner = None;
            } else {
                self.players[cell.owner.unwrap()].alive = true;
            }
            cell.num_transients = 0;
        }
        let mut continuing = false;
        for posx in 0..DIMX {
            for posy  in 0..DIMY {
                continuing |= self.spread(posx+I*posy);
            }
        }
        if continuing {
            self.steps = STEPS_TOTAL;
        } else {
            self.state = State::AcceptingInput;
            self.next_player();
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
        for (idx, player) in self.players.iter().enumerate() {
            let x = (DIMX * 100 + 50) as i16;
            let y = (30 + idx * 40) as i16;
            canvas.filled_circle(x, y, 15, player.color)?;
            canvas.aa_circle(x, y, 15, Color::RGB(0, 0, 0))?;
            if idx == self.active_player {
                canvas.filled_pie(x-20, y, 20, 160, 200, Color::RGB(0, 0, 0))?;
            }
            if !player.alive {
                canvas.thick_line(x-15, y-15, x+15, y+15, 2, Color::RGB(0, 0, 0))?;
                canvas.thick_line(x-15, y+15, x+15, y-15, 2, Color::RGB(0, 0, 0))?;
            }
        }
        for cell in self.cells.iter() {
            cell.draw(canvas)?;
        }
        for cell in self.cells.iter() {
            for marble in cell.marbles.iter() {
                marble.draw(canvas, self.players[marble.owner].color)?;
            }
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
        grid.step();
        grid.draw(&mut canvas)?;
        canvas.present();
        std::thread::sleep(Duration::new(0, 1_000_000_000u32 / 60));
    };
    Ok(())
}
