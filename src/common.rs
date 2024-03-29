use rand::{RngCore, SeedableRng};
use std::collections::{BTreeMap, VecDeque};
use std::{cmp::{PartialOrd, Ord}, fmt::Debug};
use std::ops::{Add, Index, IndexMut, Mul, Neg, Sub};
use serde::{Serialize, Deserialize};

pub const TAU: f64 = 2.0 * std::f64::consts::PI;
pub const TICKS_PER_SECOND: f64 = 2.0;

pub trait GameState {
    type PlayerInput: Serialize+for<'de>Deserialize<'de>+Copy+Clone+Debug+PartialEq+Eq+PartialOrd+Ord;
    type GameEvent: Serialize+for<'de>Deserialize<'de>+Copy+Clone+Debug+PartialEq+Eq+PartialOrd+Ord;
    type S2CMsg: Serialize+for<'de>Deserialize<'de>+Clone+Debug;
    type C2SMsg: Serialize+for<'de>Deserialize<'de>+Clone+Debug;

    fn new() -> Self;
    fn tick(&mut self, inputs: &BTreeMap<PlayerId, Self::PlayerInput>) -> Vec<Self::GameEvent>;
}

/* ===== Message types ===== */

#[derive(Serialize, Deserialize, Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum SnakePlayerInput {
    ChangeDirection(Direction),
}

#[derive(Serialize, Deserialize, Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum SnakeGameEvent {
    PlayerDied(PlayerId, u32),
    PlayerAteFood(PlayerId, Coord),
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub enum ServerToClient {
    Initialize { pid: PlayerId, world: SnakeGameState },
    DoTick { tick: u64, inputs: BTreeMap<PlayerId, SnakePlayerInput> },
    PlayerDisconnected { pid: PlayerId }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub enum ClientToServer {
    InputAtTick { tick: u64, input: SnakePlayerInput },
}

/* ===== Data structures ===== */

#[derive(Serialize, Deserialize, Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct PlayerId(pub usize);

#[derive(Serialize, Deserialize, Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Coord { x: isize, y: isize }

#[derive(Serialize, Deserialize, Copy, Clone, Debug, PartialEq)]
pub struct Vec2 { pub x: f64, pub y: f64 }

#[derive(Serialize, Deserialize, Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Direction { Up, Down, Left, Right }

#[derive(Serialize, Deserialize, Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Tile {
    Empty,
    Wall,
    WormSegment { pid: PlayerId, dir: Direction, },
    Food,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Board {
    pub width: usize,
    pub height: usize,
    pub tiles: Vec<Tile>,
}

pub mod serializable_chacha;
use serializable_chacha::SerializableChaCha20;

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct SnakeGameState {
    pub rng: SerializableChaCha20,
    pub tick: u64,
    pub board: Board,
    pub player_segments: BTreeMap<PlayerId, VecDeque<Coord>>,
    pub num_foods: u64,
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
        self.delta_vec2().round()
    }
    pub fn delta_vec2(self) -> Vec2 {
        Vec2::from_angle(self.radians())
    }
    pub fn from_u32(x: u32) -> Direction {
        use Direction::*;
        match x % 4 { 0 => Up, 1 => Down, 2 => Left, _ => Right }
    }
}

impl Neg for Direction {
    type Output = Direction;
    fn neg(self) -> Direction {
        match self {
            Direction::Right => Direction::Left,
            Direction::Down => Direction::Up,
            Direction::Left => Direction::Right,
            Direction::Up => Direction::Down,
        }
    }
}

#[test]
fn test_delta_coord() {
    use Direction::*;
    for dir in &[Up, Down, Left, Right] {
        println!("{:?}", dir.delta_coord());
    }
}

impl Vec2 {
    pub fn new(x: f64, y: f64) -> Vec2 {
        Vec2 { x, y }
    }
    pub fn round(self) -> Coord {
        Coord { x: self.x.round() as isize, y: self.y.round() as isize }
    }
    pub fn from_angle(theta: f64) -> Vec2 {
        Vec2 { x: theta.cos(), y: theta.sin() }
    }
}
impl Add<Vec2> for Vec2 {
    type Output = Vec2;
    fn add(self, rhs: Vec2) -> Vec2 {
        Vec2 { x: self.x + rhs.x, y: self.y + rhs.y }
    }
}
impl Neg for Vec2 {
    type Output = Vec2;
    fn neg(self) -> Vec2 {
        Vec2 { x: -self.x, y: -self.y}
    }
}
impl Sub<Vec2> for Vec2 {
    type Output = Vec2;
    fn sub(self, rhs: Vec2) -> Vec2 {
        Vec2 { x: self.x - rhs.x, y: self.y - rhs.y }
    }
}
impl Mul<Vec2> for Vec2 {
    type Output = Vec2;
    fn mul(self, rhs: Vec2) -> Vec2 {
        Vec2 { x: self.x * rhs.x, y: self.y * rhs.y }
    }
}
impl Mul<Vec2> for f64 {
    type Output = Vec2;
    fn mul(self, rhs: Vec2) -> Vec2 {
        Vec2 { x: self * rhs.x, y: self * rhs.y }
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
    pub fn to_vec2(self) -> Vec2 {
        Vec2::new(self.x as _, self.y as _)
    }
}

impl Add<Coord> for Coord {
    type Output = Coord;
    fn add(self, rhs: Coord) -> Coord {
        signed_coord(self.x + rhs.x, self.y + rhs.y)
    }
}
impl Neg for Coord {
    type Output = Coord;
    fn neg(self) -> Coord {
        signed_coord(-self.x, -self.y)
    }
}
impl Sub<Coord> for Coord {
    type Output = Coord;
    fn sub(self, rhs: Coord) -> Coord {
        self + (-rhs)
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

    pub fn move_head(&mut self, c: Coord) -> (Vec<SnakeGameEvent>, Option<Coord>) {
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
                    ret.0.push(SnakeGameEvent::PlayerDied(pid, (0.1 * u32::max_value() as f64) as u32));
                },
                WormSegment { pid: _, dir: _ } => {
                    ret.0.push(SnakeGameEvent::PlayerDied(pid, (0.9 * u32::max_value() as f64) as u32));
                },
                Food => {
                    self[c2] = WormSegment { pid, dir };
                    ret.0.push(SnakeGameEvent::PlayerAteFood(pid, c2));
                    ret.1 = Some(c2);
                },
            }
        }
        ret
    }
}

impl GameState for SnakeGameState {
    type PlayerInput = SnakePlayerInput;
    type GameEvent = SnakeGameEvent;
    type S2CMsg = ServerToClient;
    type C2SMsg = ClientToServer;

    fn new() -> SnakeGameState {
        SnakeGameState {
            rng: SeedableRng::seed_from_u64(0xdeadbeefdeadbeef),
            tick: 0,
            board: Board::new(40, 30),
            player_segments: BTreeMap::new(),
            num_foods: 0,
        }
    }

    fn tick(&mut self, inputs: &BTreeMap<PlayerId, Self::PlayerInput>) -> Vec<Self::GameEvent> {
        for (pid, input) in inputs.iter() {
            match input {
                SnakePlayerInput::ChangeDirection(dir) => self.change_direction(*pid, *dir),
            }
        }
        let mut events = vec![];
        for (_, segments) in self.player_segments.iter_mut() {
            if let Some(head) = segments.back() {
                let (new_events, new_segment) = self.board.move_head(*head);
                if let Some(s) = new_segment {
                    segments.push_back(s);
                }
                if segments.len() > 1 && new_events.iter().all(|e| match e { SnakeGameEvent::PlayerAteFood(_, _) => false, _ => true }) {
                    self.board[segments.pop_front().unwrap()] = Tile::Empty;
                }
                events.extend(new_events);
            }
        }
        for event in events.iter() {
            match event {
                SnakeGameEvent::PlayerDied(pid, food_probability) => self.remove_player(*pid, *food_probability),
                SnakeGameEvent::PlayerAteFood(_, _) => {
                    // TODO: score?
                    self.num_foods -= 1;
                },
            }
        }
        let n = self.player_segments.len() as u64 + 2;
        while self.num_foods < n {
            self.spawn_food();
        }
        self.tick += 1;
        events
    }
}

impl SnakeGameState {
    pub fn spawn_player(&mut self, pid: PlayerId) {
        let dir = Direction::from_u32(self.rng.next_u32());
        loop {
            let c = self.random_coord();
            // TODO: reroll location if the spawn would be in danger in 2-3 ticks
            if let Tile::Empty = self.board[c] {
                self.board[c] = Tile::WormSegment { pid, dir };
                self.player_segments.entry(pid).or_insert_with(|| VecDeque::new()).push_back(c);
                break
            }
        }
    }

    pub fn change_direction(&mut self, pid: PlayerId, dir: Direction) {
        if let Some(segments) = self.player_segments.get_mut(&pid) {
            if let Some(head) = segments.back() {
                if let Tile::WormSegment { pid: pid2, dir: dir2 } = &mut self.board[*head] {
                    assert_eq!(pid, *pid2);
                    if dir.delta_coord() + dir2.delta_coord() != coord(0, 0) {
                        *dir2 = dir;
                    }
                }
            }
        }
    }

    pub fn remove_player(&mut self, pid: PlayerId, food_probability: u32) {
        if let Some(segments) = self.player_segments.remove(&pid) {
            for segment in segments {
                self.board[segment] = if self.rng.next_u32() < food_probability { self.num_foods += 1; Tile::Food } else { Tile::Empty };
            }
        }
    }
    pub fn random_coord(&mut self) -> Coord {
        coord(self.rng.next_u32() as usize % self.board.width, self.rng.next_u32() as usize % self.board.height)
    }

    pub fn spawn_food(&mut self) {
        loop {
            let c = self.random_coord();
            if let Tile::Empty = self.board[c] {
                self.board[c] = Tile::Food;
                self.num_foods += 1;
                break
            }
        }
    }
}
