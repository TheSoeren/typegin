use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Direction {
    North,
    South,
    East,
    West,
}

impl Direction {
    pub fn parse(raw: &str) -> Option<Direction> {
        match raw {
            "n" | "north" => Some(Direction::North),
            "s" | "south" => Some(Direction::South),
            "w" | "west" => Some(Direction::West),
            "e" | "east" => Some(Direction::East),
            _ => None,
        }
    }
}

impl fmt::Display for Direction {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Direction::North => write!(f, "north"),
            Direction::East => write!(f, "east"),
            Direction::South => write!(f, "south"),
            Direction::West => write!(f, "west"),
        }
    }
}
