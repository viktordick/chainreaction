use num::complex;

use arr_macro::arr;

use crate::game::State;

pub type Point = complex::Complex<i32>;
pub type Owner = usize;

pub const DIMX: usize = 8;
pub const DIMY: usize = 6;
pub const DIM: Point = Point::new(DIMX as i32, DIMY as i32);

// main directions
pub const DIRECTIONS: [Point; 4] = [
    Point::new(1, 0),
    Point::new(0, 1),
    Point::new(-1, 0),
    Point::new(0, -1),
];

pub struct PointIter {
    p: Point,
}
impl PointIter {
    pub fn new() -> PointIter {
        PointIter {
            p: Point::new(DIM.re-1, DIM.im),
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
            self.p.im = DIM.im - 1;
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

pub struct Cell {
    coord: Point,
    owner: Option<Owner>,
    neighbors: u8,
    count: u8,
    has_neighbor: [bool; 4],
    // Residing, Incoming and Outgoing for each direction
    slots: [[Option<Marble>; 4]; 3],
}
impl Cell {
    fn new(coord: Point) -> Cell {
        let has_neighbor = [
            coord.re < DIM.re - 1,
            coord.im < DIM.im - 1,
            coord.re > 0,
            coord.im > 0,
        ];
        Cell {
            coord: coord,
            owner: None,
            has_neighbor: has_neighbor,
            slots: [[None; 4]; 3],
            neighbors: has_neighbor.into_iter().map(|x| x as u8).sum(),
            count: 0,
        }
    }

    pub fn has_neighbor(&self, direction: usize) -> bool { self.has_neighbor[direction] }
    fn residing(&self) -> &[Option<Marble>; 4] { &self.slots[0] }
    fn incoming(&self) -> &[Option<Marble>; 4] { &self.slots[1] }
    fn outgoing(&self) -> &[Option<Marble>; 4] { &self.slots[2] }
    fn residing_mut(&mut self) -> &mut [Option<Marble>; 4] { &mut self.slots[0] }
    fn incoming_mut(&mut self) -> &mut [Option<Marble>; 4] { &mut self.slots[1] }
    fn outgoing_mut(&mut self) -> &mut [Option<Marble>; 4] { &mut self.slots[2] }

    fn full(&self) -> bool {
        self.count == self.neighbors
    }

    pub fn marbles(&self) -> impl Iterator<Item=&Marble> + '_ {
        self.slots.iter().flatten().flatten()
    }

    fn marbles_mut(&mut self) -> impl Iterator<Item=&mut Marble> + '_ {
        self.slots.iter_mut().flatten().flatten()
    }

    /* Add a marble to a cell that has room for it (in first slot)
     * Returns Err variant if there is no room (should not happen) or if the owner does not match.
     */
    fn add_marble(&mut self, owner: Owner) -> Result<(), ()>{
        if *self.owner.get_or_insert(owner) != owner {
            // Set owner if it is not yet set, but return an error if it is set differently
            return Err(())
        }
        if self.count == self.neighbors {
            return Err(())
        }
        self.count += 1;
        let center = self.coord * 100 + Point::new(50, 50);
        for direction in 0..4 {
            if !self.has_neighbor[direction] || self.residing()[direction].is_some() {
                continue;
            }
            self.residing_mut()[direction].get_or_insert_with(|| 
                Marble {
                    owner: owner,
                    pos: center + 25 * DIRECTIONS[direction],
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
        self.outgoing_mut().map(|mut x| x.take())
    }

    /* Receive one marble from a neighbor */
    fn receive(&mut self, direction: usize, marble: Marble) {
        self.owner = Some(marble.owner);
        self.incoming_mut()[direction] = Some(marble);
    }

    /* This is called after all full cells have send() all marbles that are to be sent and their
     * neigbors receive()d them. The Outgoing slots are therefore empty and the Incoming slots
     * might be partially full.
     * Move all marbles from Incoming slots into Outgoing or Remaining slot, possibly changing the
     * direction to make the directions balanced.
     */
    fn sort_received(&mut self) {
        let received: u8 = self.incoming().iter().flatten().map(|_| 1).sum();
        if received == 0 {
            return;
        }
        self.count += received;
        if self.full() {
            // Collect outgoing marbles, from incoming or residing
        } else {
            // Sort incoming marbles into residing
        }
    }

    fn step(&mut self, steps: i32) {
        let center = self.coord * 100 + Point::new(50, 50);
        for direction in 0..4 {
            let target = center + 25*DIRECTIONS[direction];
            for slot in 0..3 {
                if let Some(mut marble) = self.slots[slot][direction] {
                    marble.step(target, steps);
                }
            }
        }
    }
}

pub struct Grid {
    cells: [Cell; DIMX*DIMY],
}
impl Grid {
    pub fn new() -> Grid {
        /* Initialize Grid (on the stack!) */
        let mut x: i32 = 0;
        let mut y: i32 = 0;
        Grid {
            cells: arr![Cell::new({
                let coord = Point::new(x, y);
                y += 1;
                if y == DIM.im {
                    y = 0;
                    x += 1;
                }
                coord
            }); 48],  // NOTE: This is DIMX*DIMY, but unfortunately we need a literal here
        }
    }
    
    fn idx(p: Point) -> usize {
        p.re as usize * DIMY + p.im as usize
    }

    pub fn cell(&self, p: Point) -> &Cell {
        &self.cells[Self::idx(p)]
    }

    pub fn cell_mut(&mut self, p: Point) -> &mut Cell {
        &mut self.cells[Self::idx(p)]
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
        for coord in PointIter::new() {
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
    pub fn add_marble(&mut self, coord: Point, owner: Owner) -> Result<State, ()> {
        let cell = self.cell_mut(coord);
        cell.add_marble(owner)?;
        Ok(
            if cell.full() {
                self.spread()
            } else {
                State::AcceptingInput
            }
        )
    }

    /* Perform one animation step */
    pub fn step(&mut self, state: State) -> State {
        match state {
            State::AcceptingInput => state,
            State::Animating(steps) => {
                for cell in self.cells.iter_mut() {
                    cell.step(steps);
                }
                if steps == 0 {
                    self.spread()
                } else {
                    State::Animating(steps-1)
                }
            }
        }
    }
}
