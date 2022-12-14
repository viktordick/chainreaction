extern crate sdl2; 
extern crate num;

use std::str;
use std::cmp::min;
use std::vec::Vec;
use std::time::Duration;

use num::complex;

use sdl2::video::{Window,WindowContext};
use sdl2::render::{Canvas,Texture,TextureCreator};
use sdl2::surface::Surface;
use sdl2::rect::Rect;
use sdl2::pixels::{Color,PixelFormatEnum};
use sdl2::event::Event;
use sdl2::keyboard::Keycode;
use sdl2::gfx::primitives::DrawRenderer;
use sdl2::ttf;

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

/* Color and state for each player. Once the player places their first marble, they are started. If
 * they then at some point have no more marbles, they have lost and are no longer alive.
 */
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

/* Each marble has a current position and a target position that it moves towards with each step.
 */
#[derive(Clone)]
#[derive(Copy)]
struct Marble {
    // current position (in pixels)
    pos: Complex,
    // target position we are moving to
    target: Complex,
    // The direction we are facing within the cell that holds us
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
}

struct Cell {
    owner: Option<usize>,
    slots: [Complex; 4],
    num_neighbors: usize,
    has_neighbor: [bool; 4],
    // For each direction, we have a vector of marbles. Some of them might be "transient",
    // i.e. not yet to be moved away in the current animation phase because they were just added
    // (to remove artifacts from the order in which we process the cells).
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
        let mut slots = [coord; 4];
        for direction in 0..4 {
            slots[direction] = coord * 100 + Complex::new(35, 35) + 25 * DIRECTIONS[direction];
        }
        Cell {
            owner: None,
            slots: slots,
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
            self.marbles.push(Marble::new(self.slots[direction], owner, direction));
            break;
        }
        Ok(())
    }

    fn check_overflow(&mut self) -> Vec<Marble> {
        // If we have at least one marble for each direction, push one of each direction into the
        // result and remove them from the cell itself.
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
        panic!("No direction found");
    }

    fn receive(&mut self, mut marble: Marble) {
        self.owner = Some(marble.owner);
        marble.direction = self.target_direction(marble.direction);
        marble.target = self.slots[marble.direction];
        self.marbles.push(marble);
        self.num_transients += 1;
    }
}

enum State {
    AcceptingInput,
    Animating,
}

// The board
struct Grid {
    cells: Vec<Cell>,
    state: State,
    steps: i32,
    active_player: usize,
    players: Vec<Player>,
    selected: Complex,
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
        let mut players = vec![];
        players.push(Player::new(200, 0, 0));
        players.push(Player::new(0, 150, 0));
        players.push(Player::new(0, 0, 200));
        Grid{
            cells: cells,
            state: State::AcceptingInput,
            steps: 0,
            players: players,
            active_player: 0,
            selected: Complex::new(0, 0),
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

    fn keydown(&mut self, keycode: Keycode) {
        match keycode {
            Keycode::Right =>
                self.selected.re = (self.selected.re + 1) % DIMX,
            Keycode::Left =>
                self.selected.re = (self.selected.re + DIMX - 1) % DIMX,
            Keycode::Down =>
                self.selected.im = (self.selected.im + 1) % DIMY,
            Keycode::Up =>
                self.selected.im = (self.selected.im + DIMY - 1) % DIMY,
            Keycode::Return => {
                self.click(self.selected.re, self.selected.im);
            }
            _ => return
        }
    }

    fn click(&mut self, x: i32, y: i32) {
        match self.state {
            State::AcceptingInput => (),
            _ => return
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
}

// Rendering helper. This pre-renders all required textures and copies them to the board
// accordingly.
struct Renderer<'a> {
    background: Texture<'a>,
    marbles: Vec<Texture<'a>>,
    active_marker: Texture<'a>,
    dead_marker: Texture<'a>,
    selected: Texture<'a>,
}
impl<'a> Renderer<'a> {
    // Create a canvas, allow the given CanvasDrawer function to fill it, and convert to a texture.
    fn _create_texture<CanvasDrawer>(
        creator: &'a TextureCreator<WindowContext>,
        width: u32,
        height: u32,
        draw: CanvasDrawer
    ) -> Result<Texture, String>
        where CanvasDrawer: Fn(&mut Canvas<Surface>) -> Result<(), String>
    {
        let mut canvas = Surface::new(width, height, PixelFormatEnum::RGBA8888)
            ?.into_canvas()?;
        draw(&mut canvas)?;
        Ok(creator
            .create_texture_from_surface(canvas.into_surface())
            .map_err(|e| e.to_string())?)
    }

    fn gradient(canvas: &Canvas<Surface>, cx: i16, cy: i16, color: Color) -> Result<(), String> {
        for i in 0..31 {
            let mut color = color;
            color.a = (256 - (((31-i) as u32 * 140)/32) as u16) as u8;
            let halflength = ((15*15-(i-15)*(i-15)) as f64).sqrt() as i16;
            canvas.hline(cx-halflength, cx+halflength, cy-15+i, Color::RGB(200, 200, 200))?;
            canvas.hline(cx-halflength, cx+halflength, cy-15+i, color)?;
        }
        Ok(())
    }

    fn add_coords(background: &mut Canvas<Surface>, font: &ttf::Font) -> Result<(), String> {
        let creator = background.texture_creator();
        let mut render = |character: u8, posx: i32, posy: i32| -> Result<(), String> {
            let bytes: [u8; 1] = [character];
            let s = str::from_utf8(&bytes).map_err(|e| e.to_string())?;
            let rendered = font.render(&s).blended(Color::RGB(0,0,0))
                .map_err(|e| e.to_string())?;
            let texture = rendered.as_texture(&creator)
                .map_err(|e| e.to_string())?;
            background.copy(
                &texture,
                None,
                Some(
                    Rect::new(
                        posx - rendered.width() as i32/2,
                        posy - rendered.height() as i32/2,
                        rendered.width(),
                        rendered.height()
                    )
                )
            )?;
            Ok(())
        };
        for i in 0..DIMX {
            render(65+i as u8, 100*i + 50, 20)?;
        };
        for i in 0..DIMY {
            render(49+i as u8, 15, 100*i+50)?;
        }
        Ok(())
    }

    fn new(creator: &'a TextureCreator<WindowContext>, grid: &Grid, font: &ttf::Font) -> Result<Renderer<'a>, String> {
        let black = Color::RGB(0, 0, 0);

        // Marbles
        let mut marbles = Vec::with_capacity(grid.players.len());
        for player in grid.players.iter() {
            marbles.push(
                Renderer::_create_texture(creator, 31, 31, |canvas| {
                    Renderer::gradient(&canvas, 15, 15, player.color)?;
                    Ok(())
                })?
            );
        }

        Ok(Renderer{
            background: Renderer::_create_texture(
                creator, 100*DIMX as u32 + 100, 100*DIMY as u32,
                |mut canvas| {
                    canvas.set_draw_color(Color::RGB(200, 200, 200));
                    canvas.clear();
                    Renderer::add_coords(&mut canvas, &font)?;
                    for x in 0..DIMX + 1 {
                        canvas.vline((x*100) as i16, 0, 100*DIMY as i16, black)?;
                    }
                    for y in 0..DIMY {
                        canvas.hline(0, (100*DIMX) as i16, (y*100) as i16, black)?;
                    }
                    for x in 0..DIMX {
                        for y in 0..DIMY {
                            let cell = grid.cell(x+y*I);
                            let center = x*100 + 50 + (y*100+50)*I;
                            for direction in 0..4 {
                                if !cell.has_neighbor[direction] {
                                    continue
                                }
                                let pos = center + 25*DIRECTIONS[direction];
                                let cx = pos.re as i16;
                                let cy = pos.im as i16;
                                Renderer::gradient(&canvas, cx, cy, Color::RGB(255, 255, 255))?;
                            }
                        }
                    }

                    for (idx, player) in grid.players.iter().enumerate() {
                        let x = (DIMX * 100 + 50) as i16;
                        let y = (30 + idx * 40) as i16;
                        Renderer::gradient(&canvas, x, y, player.color)?;
                    }
                    Ok(())
                },
            )?,
            marbles: marbles,
            active_marker: Renderer::_create_texture(
                creator, 31, 31, |canvas| {
                    canvas.filled_pie(25, 15, 20, 160, 200, black)?;
                    Ok(())
                },
            )?,
            dead_marker: Renderer::_create_texture(
                creator, 31, 31, |canvas| {
                    canvas.thick_line(0, 0, 30, 30, 3, black)?;
                    canvas.thick_line(0, 30, 30, 0, 3, black)?;
                    Ok(())
                },
            )?,
            selected: Renderer::_create_texture(
                creator, 100, 100, |canvas| {
                    canvas.thick_line(1, 1, 100, 1, 2, black)?;
                    canvas.thick_line(1, 1, 1, 100, 2, black)?;
                    canvas.thick_line(100, 1, 100, 100, 2, black)?;
                    canvas.thick_line(1, 100, 100, 100, 2, black)?;
                    Ok(())
                },
            )?,
        })
    }

    fn update(&self, canvas: &mut Canvas<Window>, grid: &Grid) -> Result<(), String>{
        canvas.copy(&self.background, None, None)?;
        for cell in grid.cells.iter() {
            for marble in cell.marbles.iter() {
                let rect = Rect::new(marble.pos.re, marble.pos.im, 31, 31);
                canvas.copy(
                    &self.marbles[marble.owner],
                    None,
                    Some(rect),
                )?
            }
        }
        let rect = Rect::new(DIMX*100 + 5, grid.active_player as i32*40 + 15, 30, 31);
        canvas.copy(
            &self.active_marker,
            None,
            Some(rect),
        )?;
        for (idx, player) in grid.players.iter().enumerate() {
            if player.alive {
                continue
            }
            let rect = Rect::new(DIMX*100+35, 15+idx as i32*40, 31, 31);
            canvas.copy(
                &self.dead_marker,
                None,
                Some(rect),
            )?;
        }
        let x = grid.selected.re as i32;
        let y = grid.selected.im as i32;
        canvas.copy(
            &self.selected,
            None,
            Some(Rect::new(x*100, y*100, 100, 100)),
        )?;

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

    let fontcontext = ttf::init().map_err(|e| e.to_string())?;
    let font = fontcontext.load_font("/usr/share/fonts/liberation/LiberationMono-Regular.ttf", 24)?;
 
    let mut canvas = window
        .into_canvas()
        .present_vsync()
        .accelerated()
        .build()
        .map_err(|e| e.to_string())?;

    let mut grid = Grid::new();

    let texture_creator = canvas.texture_creator();
    let renderer = Renderer::new(&texture_creator, &grid, &font)?;

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
                Event::KeyDown { keycode, .. } => grid.keydown(keycode.unwrap()),
                Event::MouseButtonDown {x, y, .. } => {
                    let x = x/100;
                    let y = y/100;
                    if x < DIMX && y < DIMY {
                        grid.click(x, y);
                    }
                },
                _ => {}
            }
        }
        grid.step();
        renderer.update(&mut canvas, &grid)?;
        canvas.present();
        std::thread::sleep(Duration::new(0, 1_000_000_000u32 / 60));
    };
    Ok(())
}
