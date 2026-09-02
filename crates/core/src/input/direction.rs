use std::fmt;

/// A compass direction the player can move in.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Direction {
    /// Moves the player "up" the screen / north.
    North,
    /// Moves the player "down" the screen / south.
    South,
    /// Moves the player to the right of the screen / east.
    East,
    /// Moves the player to the left of the screen / west.
    West,
}

/// Outcome of revealing or hiding an exit in a given direction.
///
/// [`Found`](Self::Found) carries the direction that was affected (it is a
/// copy of the one passed in), signalling success; [`NotFound`](Self::NotFound)
/// means no exit was present to act on.
#[derive(Debug, PartialEq, Eq)]
pub enum DirectionResolution {
    /// An exit was present in `direction` and was revealed/hidden.
    Found(Direction),
    /// There was no exit in that direction to act on.
    NotFound,
}

impl Direction {
    /// Parse a player-typed direction string into a [`Direction`].
    ///
    /// Accepts the full word or its one-letter abbreviation
    /// (`n`/`north`, `s`/`south`, `e`/`east`, `w`/`west`), case-sensitively.
    /// Returns [`None`] for anything else.
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
