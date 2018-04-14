use std::fmt;

#[derive(Clone)]
pub struct Cell {
    pub cell_type: usize,
    pub volume: f32,
}

impl Cell {
    pub fn empty() -> Self {
        Cell {
            cell_type: 0,
            volume: 0.0,
        }
    }
}

impl fmt::Debug for Cell {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.volume)
    }
}
