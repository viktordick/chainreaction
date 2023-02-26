use std::vec::Vec;

use sdl2::keyboard::Keycode;
use sdl2::pixels::Color;

use crate::grid::{Owner, Point, Grid};
use crate::menu::Config;

/* Color and state for each player. Once the player places their first marble, they are started. If
 * they then at some point have no more marbles, they have lost and are no longer alive.
 */
pub struct Player {
    pub started: bool,
    pub alive: bool,
    color: Color,
}
impl Player {
    pub fn new(color: Color) -> Player {
        Player{
            started: false,
            alive: true,
            color: color,
        }
    }
    pub fn color(&self) -> Color { self.color }
}

#[derive(Clone,Copy, Debug)]
pub enum State {
    AcceptingInput,
    Animating(i32), // number of steps for animation
}

pub struct Game {
    players: Vec<Player>,
    state: State,
    cur_player: Owner,
    selected: Point,
    grid: Grid,
}

impl Game {
    pub fn players(&self) -> impl Iterator<Item=&Player> { self.players.iter() }
    pub fn num_players(&self) -> usize { self.players.len() }
    pub fn cur_player(&self) -> Owner { self.cur_player }
    pub fn grid(&self) -> &Grid { &self.grid }
    pub fn selected(&self) -> Point { self.selected }
    pub fn dim(&self) -> Point { self.grid.dim() }

    pub fn new(config: Config) -> Game {
        Game {
            players: config.players,
            cur_player: 0,
            state: State::AcceptingInput,
            grid: Grid::new(config.size),
            selected: Point::new(0, 0),
        }
    }

    pub fn keydown(&mut self, keycode: Keycode) {
        let dim = self.grid.dim();
        match keycode {
            Keycode::Right =>
                self.selected.re = (self.selected.re + 1) % dim.re,
            Keycode::Left =>
                self.selected.re = (self.selected.re + dim.re - 1) % dim.re,
            Keycode::Down =>
                self.selected.im = (self.selected.im + 1) % dim.im,
            Keycode::Up =>
                self.selected.im = (self.selected.im + dim.im - 1) % dim.im,
            Keycode::Return => {
                self.click(self.selected);
            }
            _ => return
        }
    }

    pub fn click(&mut self, p: Point) {
        self.selected = p;
        match self.state {
            State::AcceptingInput => (),
            _ => return
        }

        let cur_player = self.cur_player;
        self.players[cur_player].started = true;
        match self.grid.add_marble(p, cur_player) {
            Ok(state) => {
                self.state = state;
                self.next_player_if_accepting();
            },
            Err(_) => {}
        }
    }

    pub fn step(&mut self) {
        match self.state {
            State::AcceptingInput => (),
            _ => {
                self.state = self.grid.step(self.state);
                self.grid.check_players(&mut self.players);
                self.next_player_if_accepting();
            }
        }
    }

    fn next_player_if_accepting(&mut self) {
        match self.state {
            State::AcceptingInput => {
                loop {
                    self.cur_player = (self.cur_player + 1) % self.players.len();
                    if self.players[self.cur_player].alive {
                        break;
                    }
                }
            },
            _ => ()
        };
    }
}
