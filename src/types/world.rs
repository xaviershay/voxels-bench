use types::*;

#[derive(Debug)]
pub struct World {
    size: V3I,
    cells: Vec<Cell>,
    pub physics_fps: f32,
}

impl Clone for World {
    fn clone(&self) -> Self {
        World {
            size: self.size,
            cells: self.cells.clone(),
            physics_fps: self.physics_fps,
        }
    }
}

impl World {
    pub fn create(world: V3I) -> Self {
        let data = vec![Cell::empty(); (world.x * world.y * world.z) as usize];
        World { size: world, cells: data, physics_fps: 0.0 }
    }

    pub fn get(&self, v: V3I) -> Option<&Cell> {
        let size = &self.size;

        if v.x < size.x && v.y < size.y && v.z < size.z && v.x >= 0 && v.y >= 0 && v.z >= 0 {
            return Some(self.get_unsafe(v));
        } else {
            return None;
        }
    }

    pub fn get_unsafe(&self, v: V3I) -> &Cell {
        let size = &self.size;

        return &self.cells[(v.x*size.y*size.z + v.y*size.z + v.z) as usize];
    }

    pub fn update<F>(&mut self, v: V3I, mut f: F)
        where F: FnMut(&mut Cell) {

        let size = &self.size;

        if v.x < size.x && v.y < size.y && v.z < size.z && v.x >= 0 && v.y >= 0 && v.z >= 0 {
            let ref mut current = &mut self.cells[(v.x*size.y*size.z + v.y*size.z + v.z) as usize];

            f(current);
        }
    }
}
