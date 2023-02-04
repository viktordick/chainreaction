use num::complex;

type Point = complex::Complex<i32>;
type Owner = usize;

// main directions
const DIRECTIONS: [Point; 4] = [
    Point::new(1, 0),
    Point::new(0, 1),
    Point::new(-1, 0),
    Point::new(0, -1),
];

struct Marble {
    // Absolute position in pixels
    pos: Point,
    // Which owner the marble belongs to
    owner: Owner,
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
    fn new(coord: Point, dims: Point) -> Cell {
        let has_neighbor = [
            coord.re < dims.re-1,
            coord.im < dims.im-1,
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

    fn full(&self) -> bool {
        self.count == self.neighbors
    }

    /* Remove and return one marble that is to be sent into the given direction */
    fn send(&mut self, direction: usize) -> Option<Marble> {
        let result = match &mut self.slots[direction] {
            None => None,
            Some(slot) => slot.marbles[1].take(),
        };
        if result.is_some() {
            self.count -= 1;
        };
        result
    }

    /* Receive one marble from a neighbor */
    fn receive(&mut self, direction: usize, marble: Marble) {
        self.owner = Some(marble.owner);
        self.slots[direction].as_mut().unwrap().marbles[2].insert(marble);
        self.count += 1;
    }

    /* This is called after all full cells have send() all marbles that are to be sent and their
     * neigbors receive()d them. The Outgoing slots are therefore empty and the Incoming slots
     * might be partially full.
     * Move all marbles from Incoming slots into Outgoing or Remaining slot, possibly changing the
     * direction to make the directions balanced.
     */
    fn sort_receiving(&mut self) {
        let received: u8 = self.slots.iter().flatten()
            .map(|x| &x.marbles[2]).flatten().map(|_| 1).sum();
        if (received == 0) {
            return;
        }
        
        // TODO
    }



    /* Sort all incoming marbles into other slots, changing the owner of all marbles if any marbles
     * arrived.
     * Returns if the cell is full afterwards
     * */
    fn sort_incoming(&mut self) -> bool {
        let mut chown = false;
        for marble in self.slots.iter().flatten().map(|x| &x.marbles[2]).flatten() {
            if self.owner != Some(marble.owner) {
                chown = true;
                self.owner = Some(marble.owner);
                break;
            }
        };
        if chown {
            for mut marble in self.slots.iter_mut().flatten()
                .map(|x| &mut x.marbles).flatten().flatten() {
                    marble.owner = self.owner.unwrap();
                }
        };
        for mut slots in self.slots.iter_mut().flatten() {
            match slots.marbles[2].take() {
                None => continue,
                Some(mut marble) => {
                    match slots.marbles[0] {
                        None => slots.marbles[0].insert(marble),
                        Some(_) => slots.marbles[1].insert(marble),
                    };
                }
            }
        }
        self.count == self.neighbors
    }

}
