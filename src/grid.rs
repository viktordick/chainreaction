use num::complex;

use arr_macro::arr;

type Point = complex::Complex<i32>;
type Owner = usize;

enum State {
    AcceptingInput,
    Animating(i32), // number of steps for animation
}

const DIMX: usize = 8;
const DIMY: usize = 6;

// main directions
const DIRECTIONS: [Point; 4] = [
    Point::new(1, 0),
    Point::new(0, 1),
    Point::new(-1, 0),
    Point::new(0, -1),
];

struct PointIter {
    p: Point,
}
impl PointIter {
    fn new() -> PointIter {
        PointIter {
            p: Point::new(0, 0),
        }
    }
}
impl Iterator for PointIter {
    type Item = Point;
    fn next(&mut self) -> Option<Self::Item> {
        if self.p.re == DIMX as i32{
            return None;
        }
        self.p.im += 1;
        if self.p.im == DIMY as i32{
            self.p.im = 0;
            self.p.re += 1
        }
        if self.p.re >= DIMX as i32 {
            None
        } else {
            Some(self.p)
        }
    }
}


#[derive(Clone,Copy)]
struct Marble {
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
}

/* Each direction holds different slots for marbles */
struct Slots {
    // Residing, Incoming and Outgoing slot.
    marbles: [Option<Marble>; 3],
}
impl Slots {
    fn new(has_neighbor: bool) -> Option<Slots>{
        if has_neighbor {
            Some(Slots {marbles: [None, None, None]})
        } else {
            None
        }
    }
}

struct Cell {
    coord: Point,
    owner: Option<Owner>,
    neighbors: u8,
    count: u8,
    // Some slots if there is a neighbor in that direction, else None
    slots: [Option<Slots>; 4],
}
impl Cell {
    fn new(coord: Point) -> Cell {
        let has_neighbor = [
            coord.re < DIMX as i32 - 1,
            coord.im < DIMY as i32 - 1,
            coord.re > 0,
            coord.im > 0,
        ];
        Cell {
            coord: coord,
            owner: None,
            slots: has_neighbor.map(Slots::new),
            neighbors: has_neighbor.into_iter().map(|x| x as u8).sum(),
            count: 0,
        }
    }

    fn full(&self) -> bool {
        self.count == self.neighbors
    }

    fn marbles(&self) -> impl Iterator<Item=&Marble> + '_ {
        self.slots.iter().flatten().map(
            |slots: &Slots| slots.marbles.iter().flatten()
        ).flatten()
    }

    fn marbles_mut(&mut self) -> impl Iterator<Item=&mut Marble> + '_ {
        self.slots.iter_mut().flatten().map(
            |slots: &mut Slots| slots.marbles.iter_mut().flatten()
        ).flatten()
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
            if self.slots[direction].is_none() {
                continue;
            }
            let mut slots = &mut self.slots[direction].as_mut().unwrap();
            if slots.marbles[0].is_some() {
                continue;
            }
            slots.marbles[0] = Some(
                Marble {
                    owner: owner,
                    pos: center + 25 * DIRECTIONS[direction],
                }
            );
            break
        }
        Ok(())
    }

    /* Remove and return one marble from each direction that is to be sent */
    fn send(&mut self) -> [Option<Marble>; 4] {
        let mut result = [None; 4];
        for direction in 0..4 {
            match &mut self.slots[direction] {
                None => (),
                Some(slot) => {
                    result[direction] = slot.marbles[1].take();
                    self.count -= 1;
                }
            }
        }
        result
    }

    /* Receive one marble from a neighbor */
    fn receive(&mut self, direction: usize, marble: Marble) {
        self.owner = Some(marble.owner);

        self.count += 1;
    }

    /* This is called after all full cells have send() all marbles that are to be sent and their
     * neigbors receive()d them. The Outgoing slots are therefore empty and the Incoming slots
     * might be partially full.
     * Move all marbles from Incoming slots into Outgoing or Remaining slot, possibly changing the
     * direction to make the directions balanced.
     */
    fn sort_received(&mut self) {
        let received: u8 = self.slots.iter().flatten()
            .map(|x| &x.marbles[2]).flatten().map(|_| 1).sum();
        if received == 0 {
            return;
        }
        
        // TODO
    }

    fn step(&mut self, steps: i32) {
        let center = self.coord * 100 + Point::new(50, 50);
        for (direction, slots) in self.slots.iter_mut().enumerate() {
            match slots {
                None => (),
                Some(slots) => {
                    let target = center + 25*DIRECTIONS[direction];
                    for mut marble in slots.marbles.iter_mut().flatten() {
                        marble.step(target, steps);
                    }
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
        let mut x: usize = 0;
        let mut y: usize = 0;
        Grid {
            cells: arr![Cell::new({
                let coord = Point::new(x as i32, y as i32);
                y += 1;
                if y == DIMY {
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

    fn cell(&self, p: Point) -> &Cell {
        &self.cells[Self::idx(p)]
    }

    fn cell_mut(&mut self, p: Point) -> &mut Cell {
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
        let mut cell = self.cell_mut(coord / 100);
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
