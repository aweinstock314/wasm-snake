use std::collections::HashMap;
use std::ops::{Add, Index, IndexMut};

pub const TAU: f64 = 2.0 * std::f64::consts::PI;
pub const TICKS_PER_SECOND: f64 = 2.0;

/* ===== Data structures ===== */

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct PlayerId(pub usize);

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum PlayerInput {
    ChangeDirection(Direction),
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum GameEvent {
    PlayerDied(PlayerId),
    PlayerAteFood(PlayerId, Coord),
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct Coord { x: isize, y: isize }

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum Direction { Up, Down, Left, Right }

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
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

#[derive(Debug)]
pub struct GameState {
    pub board: Board,
    pub player_segments: HashMap<PlayerId, Vec<Coord>>,
}


/* ===== Methods ===== */

impl Direction {
    pub fn radians(self) -> f64 {
        use Direction::*;
        match self {
            Right => 0.0,
            Down => TAU / 4.0,
            Left => TAU / 2.0,
            Up => 3.0 * TAU / 4.0,
        }
    }
    pub fn delta_coord(self) -> Coord {
        let angle = self.radians();
        signed_coord(angle.cos().round() as _, angle.sin().round() as _)
    }
}

#[test]
fn test_delta_coord() {
    use Direction::*;
    for dir in &[Up, Down, Left, Right] {
        println!("{:?}", dir.delta_coord());
    }
}


pub fn coord(x: usize, y: usize) -> Coord {
    Coord { x: x as _, y: y as _ }
}

pub fn signed_coord(x: isize, y: isize) -> Coord {
    Coord { x: x, y: y }
}

impl Coord {
    pub fn offset(self, dir: Direction) -> Coord {
        self + dir.delta_coord()
    }
}

impl Add<Coord> for Coord {
    type Output = Coord;
    fn add(self, rhs: Coord) -> Coord {
        signed_coord(self.x + rhs.x, self.y + rhs.y)
    }
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
        c.y as usize * self.width + c.x as usize
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

    pub fn move_head(&mut self, c: Coord) -> (Vec<GameEvent>, Option<Coord>) {
        use Tile::*;
        let mut ret = (vec![], None);
        if let WormSegment { pid, dir } = self[c] {
            let c2 = c.offset(dir);
            match self[c2] {
                Empty => {
                    self[c2] = WormSegment { pid, dir };
                    ret.1 = Some(c2);
                },
                Wall => {
                    ret.0.push(GameEvent::PlayerDied(pid));
                },
                WormSegment { pid: _, dir: _ } => {
                    ret.0.push(GameEvent::PlayerDied(pid));
                },
                Food => {
                    self[c2] = WormSegment { pid, dir };
                    ret.0.push(GameEvent::PlayerAteFood(pid, c2));
                    ret.1 = Some(c2);
                },
            }
        }
        ret
    }
}

impl GameState {
    pub fn new() -> GameState {
        GameState {
            board: Board::new(40, 30),
            player_segments: HashMap::new(),
        }
    }
    pub fn place_player_near(&mut self, c: Coord, pid: PlayerId) {
        // TODO: reroll location if the spawn would be in danger, chose direction pseudorandomly
        let dir = Direction::Right;
        self.board[c] = Tile::WormSegment { pid, dir };
        self.player_segments.entry(pid).or_insert_with(|| vec![]).push(c);
    }

    pub fn change_direction(&mut self, pid: PlayerId, dir: Direction) {
        if let Some(segments) = self.player_segments.get_mut(&pid) {
            if let Some(head) = segments.last() {
                if let Tile::WormSegment { pid: pid2, dir: dir2 } = &mut self.board[*head] {
                    assert_eq!(pid, *pid2);
                    *dir2 = dir;
                }
            }
        }
    }

    pub fn remove_player(&mut self, pid: PlayerId) {
        if let Some(segments) = self.player_segments.remove(&pid) {
            for segment in segments {
                self.board[segment] = Tile::Empty;
            }
        }
    }

    pub fn tick(&mut self, inputs: &HashMap<PlayerId, PlayerInput>) -> Vec<GameEvent> {
        for (pid, input) in inputs.iter() {
            match input {
                PlayerInput::ChangeDirection(dir) => self.change_direction(*pid, *dir),
            }
        }
        let mut events = vec![];
        for (pid, segments) in self.player_segments.iter_mut() {
            if let Some(head) = segments.last() {
                let (new_events, new_segment) = self.board.move_head(*head);
                if let Some(s) = new_segment {
                    segments.push(s);
                }
                if segments.len() > 1 && new_events.iter().all(|e| match e { GameEvent::PlayerAteFood(_, _) => false, _ => true }) {
                    self.board[segments.remove(0)] = Tile::Empty;
                }
                events.extend(new_events);
            }
        }
        for event in events.iter() {
            match event {
                GameEvent::PlayerDied(pid) => self.remove_player(*pid),
                GameEvent::PlayerAteFood(pid, coord) => {
                    // TODO: score?
                },
            }
        }
        events
    }
}
