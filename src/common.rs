use std::ops::{Index, IndexMut};

pub const TAU: f64 = 2.0 * std::f64::consts::PI;

#[derive(Copy, Clone, Debug)]
pub struct PlayerId(pub usize);

#[derive(Copy, Clone, Debug)]
pub struct Coord { x: usize, y: usize }

#[derive(Copy, Clone, Debug)]
pub enum Direction { Up, Down, Left, Right }

#[derive(Copy, Clone, Debug)]
pub enum Tile {
    Empty,
    Wall,
    WormSegment { pid: PlayerId, dir: Direction, },
    Food,
}

#[derive(Debug)]
pub struct Board {
    pub width: usize,
    pub height: usize,
    pub tiles: Vec<Tile>,
}

impl Direction {
    pub fn radians(&self) -> f64 {
        use Direction::*;
        match self {
            Right => 0.0,
            Up => TAU / 4.0,
            Left => TAU / 2.0,
            Down => 3.0 * TAU / 4.0,
        }
    }
}

pub fn coord(x: usize, y: usize) -> Coord {
    Coord { x, y }
}

impl Index<Coord> for Board {
    type Output = Tile;
    fn index(&self, c: Coord) -> &Tile {
        let idx = self.idx_of_coord(c);
        &self.tiles[idx]
    }
}
impl IndexMut<Coord> for Board {
    fn index_mut(&mut self, c: Coord) -> &mut Tile {
        let idx = self.idx_of_coord(c);
        &mut self.tiles[idx]
    }
}

impl Board {
    pub fn idx_of_coord(&self, c: Coord) -> usize {
        c.y * self.width + c.x
    }

    pub fn new(width: usize, height: usize) -> Board {
        let tiles = vec![Tile::Empty; width * height];
        let mut ret = Board { width, height, tiles };
        for i in 0..width {
            ret[coord(i, 0)] = Tile::Wall;
            ret[coord(i, height-1)] = Tile::Wall;
        }
        for i in 0..height {
            ret[coord(0, i)] = Tile::Wall;
            ret[coord(width-1, i)] = Tile::Wall;
        }
        ret
    }

    pub fn place_food(&mut self, x: usize, y: usize) {
        self[coord(x, y)] = Tile::Food;
    }

    pub fn place_player(&mut self, x: usize, y: usize, pid: PlayerId, dir: Direction) {
        self[coord(x, y)] = Tile::WormSegment { pid, dir };
    }
}
