use std::ops::{Index,IndexMut};

use num_complex::Complex;

use array_macro::array;

use crate::game::{State, Player};

pub type Point = Complex<i32>;
pub type Owner = usize;

// main directions
pub const DIRECTIONS: [Point; 4] = [
    Point::new(1, 0),
    Point::new(0, 1),
    Point::new(-1, 0),
    Point::new(0, -1),
];

pub struct PointIter {
    dim: Point,
    p: Point,
}
impl PointIter {
    pub fn new(dim: Point) -> PointIter {
        PointIter {
            dim: dim,
            p: Point::new(dim.re-1, dim.im),
        }
    }
}
impl Iterator for PointIter {
    type Item = Point;
    fn next(&mut self) -> Option<Self::Item> {
        if self.p.im != 0 {
            self.p.im -= 1;
            Some(self.p)
        } else {
            self.p.re -= 1;
            self.p.im = self.dim.im - 1;
            if self.p.re >= 0 {
                Some(self.p)
            } else {
                None
            }
        }
    }
}


#[derive(Clone,Copy)]
pub struct Marble {
    // Absolute position in pixels
    pos: Point,
    // Which owner the marble belongs to
    owner: Owner,
}
impl Marble {
    /* Move one step towards target, with 'steps' remaining steps afterwards */
    fn step(&mut self, target: Point, steps: i32) {
        self.pos = target + ((self.pos - target) * steps) / (steps + 1);
    }
    pub fn get_owner(&self) -> Owner {
        self.owner
    }
    pub fn get_pos(&self) -> Point {
        self.pos
    }
}

// One set of slots, with up to one marble per direction. Residing, Incoming or Outgoing
struct Slots {
    marbles: [Option<Marble>; 4]
}
impl Slots {
    fn new() -> Slots {
        Slots {
            marbles: [None; 4]
        }
    }
}
impl Index<usize> for Slots {
    type Output = Option<Marble>;
    fn index(&self, idx: usize) -> &Self::Output {
        &self.marbles[idx]
    }
}
impl IndexMut<usize> for Slots {
    fn index_mut(&mut self, idx: usize) -> &mut Self::Output {
        &mut self.marbles[idx]
    }
}

pub struct Cell {
    coord: Point,
    owner: Option<Owner>,
    neighbors: u8,
    count: u8,
    has_neighbor: [bool; 4],
    // Residing, Incoming and Outgoing for each direction
    slots: [Slots; 3],
}
impl Cell {
    fn new(coord: Point, dim: Point) -> Cell {
        let has_neighbor = [
            coord.re < dim.re - 1,
            coord.im < dim.im - 1,
            coord.re > 0,
            coord.im > 0,
        ];
        Cell {
            coord: coord,
            owner: None,
            has_neighbor: has_neighbor,
            slots: array![_ => Slots::new(); 3],
            neighbors: has_neighbor.into_iter().map(|x| x as u8).sum(),
            count: 0,
        }
    }

    pub fn has_neighbor(&self, direction: usize) -> bool { self.has_neighbor[direction] }
    fn residing(&self) -> &Slots { &self.slots[0] }
    fn incoming(&self) -> &Slots { &self.slots[1] }
    fn outgoing(&self) -> &Slots { &self.slots[2] }
    fn residing_mut(&mut self) -> &mut Slots { &mut self.slots[0] }
    fn incoming_mut(&mut self) -> &mut Slots { &mut self.slots[1] }
    fn outgoing_mut(&mut self) -> &mut Slots { &mut self.slots[2] }

    fn full(&self) -> bool {
        self.count >= self.neighbors
    }

    pub fn marbles(&self) -> impl Iterator<Item=&Marble> + '_ {
        self.slots.iter().map(
            |slots: &Slots| slots.marbles.iter().flatten()
        ).flatten()
    }

    fn marbles_mut(&mut self) -> impl Iterator<Item=&mut Marble> + '_ {
        self.slots.iter_mut().map(
            |slots: &mut Slots| slots.marbles.iter_mut().flatten()
        ).flatten()
    }

    /* Add a marble to a cell that has room for it (in first slot)
     * Returns Err variant if there is no room (should not happen) or if the owner does not match.
     */
    fn add_marble(&mut self, owner: Owner, cellsize: i32) -> Result<(), ()>{
        if *self.owner.get_or_insert(owner) != owner {
            // Set owner if it is not yet set, but return an error if it is set differently
            return Err(())
        }
        if self.count == self.neighbors {
            return Err(())
        }
        self.count += 1;
        let center = self.coord * cellsize + Point::new(cellsize/2, cellsize/2);
        for direction in 0..4 {
            if !self.has_neighbor[direction] || self.residing()[direction].is_some() {
                continue;
            }
            self.residing_mut()[direction].get_or_insert_with(|| 
                Marble {
                    owner: owner,
                    pos: center + cellsize/4 * DIRECTIONS[direction],
                }
            );
            break
        }
        if self.full() {
            for direction in 0..4 {
                if let Some(marble) = self.residing_mut()[direction].take() {
                    self.outgoing_mut()[direction] = Some(marble);
                }
            }
        }
        Ok(())
    }

    /* Remove and return one marble from each direction that is to be sent */
    fn send(&mut self) -> [Option<Marble>; 4] {
        let mut result = [None; 4];
        for idx in 0..4 {
            result[idx] = self.outgoing_mut()[idx].take();
            if result[idx].is_some() {
                self.count -= 1;
            }
        }
        if self.count == 0 {
            self.owner = None;
        }
        result
    }

    /* Receive one marble from a neighbor */
    fn receive(&mut self, direction: usize, marble: Marble) {
        self.owner = Some(marble.owner);
        self.incoming_mut()[direction] = Some(marble);
        self.count += 1;
    }

    /* This is called after all full cells have send() all marbles that are to be sent and their
     * neigbors receive()d them. The Outgoing slots are therefore empty and the Incoming slots
     * might be partially full.
     * Move all marbles from Incoming slot into Outgoing or Remaining slot, possibly changing the
     * direction to make the directions balanced.
     */
    fn sort_received(&mut self) {
        let mut received = false;
        for _ in self.incoming().marbles {
            received = true;
        }
        if !received {
            return;
        }
        if self.full() {
            // Collect outgoing marbles, from incoming or residing
            for direction in 0..4 {
                self.outgoing_mut()[direction] = self.incoming_mut()[direction].take();
            }
            for rotation in [0, 1, 3, 2] {
                for direction in 0..4 {
                    if !self.has_neighbor[direction] || self.outgoing()[direction].is_some() {
                        continue
                    };
                    self.outgoing_mut()[direction] = self.residing_mut()[(direction+rotation)%4].take();
                }
            }
        } else {
            // Sort incoming marbles into residing
            for rotation in [0, 1, 3, 2] {
                for direction in 0..4 {
                    if !self.has_neighbor[direction] || self.residing()[direction].is_some() {
                        continue
                    };
                    self.residing_mut()[direction] = self.incoming_mut()[(direction+rotation)%4].take();
                }
            }
        }
    }

    fn step(&mut self, steps: i32, cellsize: i32) {
        let center = self.coord * cellsize + Point::new(cellsize/2, cellsize/2);
        for direction in 0..4 {
            let target = center + cellsize/4 *DIRECTIONS[direction];
            for slot in 0..3 {
                if let Some(marble) = self.slots[slot][direction].as_mut() {
                    marble.step(target, steps);
                }
            }
        }
    }
}

pub struct Grid {
    dim: Point,
    cells: Vec<Cell>,
}
impl Grid {
    pub fn new(dim: Point) -> Grid {
        let mut cells: Vec<Cell> = Vec::with_capacity(dim.re as usize * dim.im as usize);
        for x in 0..dim.re {
            for y in 0..dim.im {
                cells.push(Cell::new(Point::new(x as i32, y as i32), dim));
            }
        }
        Grid {
            dim: dim,
            cells: cells,
        }
    }
    pub fn dim(&self) -> Point { self.dim }
    
    fn idx(&self, p: Point) -> usize {
        (p.re * self.dim.im + p.im) as usize
    }

    pub fn cell(&self, p: Point) -> &Cell {
        &self.cells[self.idx(p)]
    }

    pub fn cell_mut(&mut self, p: Point) -> &mut Cell {
        let idx = self.idx(p);
        &mut self.cells[idx]
    }

    /* After a adding a marble that fills the field or at the end of an animation, this is called
     * to move marbles from full cells to their neighbors.
     * This does not directly change the position of the marbles, but it changes what cell they
     * belong to, which determines their target position. The owner of the neighboring cells is
     * also changed, but the owner of the already existing marbles is changed at the start of the
     * next call to spread().
     */
    fn spread(&mut self) -> State {
        // Change ownership of marbles
        for cell in self.cells.iter_mut() {
            match cell.owner {
                None => (),
                Some(owner) => {
                    for marble in cell.marbles_mut() {
                        marble.owner = owner;
                    }
                }
            }
        }
        // Spread out
        let mut any_moved = false;
        for coord in PointIter::new(self.dim) {
            if !self.cell(coord).full() {
                continue
            }
            let sent = self.cell_mut(coord).send();

            for direction in 0..4 {
                match sent[direction] {
                    None => continue,
                    Some(marble) => {
                        let neighbor = self.cell_mut(coord + DIRECTIONS[direction]);
                        neighbor.receive((direction+2)%4, marble);
                        any_moved = true;
                    }
                }
            }
        }
        if any_moved {
            for cell in self.cells.iter_mut() {
                cell.sort_received();
            }
            State::Animating(15)
        } else {
            State::AcceptingInput
        }
    }

    pub fn marbles(&self) -> impl Iterator<Item=&Marble> + '_ {
        self.cells.iter().map(
            |cell: &Cell| cell.marbles()
        ).flatten()
    }

    /* Try to add a marble at the given coordinates.
     * Returns the Err variant if the cell belongs to someone else.
     * May be called in AcceptingInput state.
     */
    pub fn add_marble(&mut self, coord: Point, owner: Owner, cellsize: i32) -> Result<State, ()> {
        let cell = self.cell_mut(coord);
        cell.add_marble(owner, cellsize)?;
        Ok(
            if cell.full() {
                self.spread()
            } else {
                State::AcceptingInput
            }
        )
    }

    /* Perform one animation step */
    pub fn step(&mut self, state: State, cellsize: i32) -> State {
        match state {
            State::AcceptingInput => state,
            State::Animating(steps) => {
                for cell in self.cells.iter_mut() {
                    cell.step(steps, cellsize);
                }
                if steps == 0 {
                    self.spread()
                } else {
                    State::Animating(steps-1)
                }
            }
        }
    }

    // Check which players are no longer alive
    pub fn check_players(&self, players: &mut Vec<Player>) {
        for player in players.iter_mut() {
            if player.started {
                player.alive = false;
            }
        }
        for cell in self.cells.iter() {
            if let Some(owner) = cell.owner {
                players[owner].started = true;
                players[owner].alive = true;
            }
        }
    }
}
