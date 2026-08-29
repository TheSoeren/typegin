#[derive(Debug, PartialEq, Eq)]
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
