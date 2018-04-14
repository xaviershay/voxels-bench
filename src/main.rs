use std::fmt;
use std::mem::size_of;
use std::time::{Instant};

#[derive(Clone, Copy)]
struct V3I {
    x: i32,
    y: i32,
    z: i32,
}

impl V3I {
    fn zero() -> Self {
        V3I { x: 0, y: 0, z: 0 }
    }
}

impl fmt::Debug for V3I {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "({}, {}, {})", self.x, self.y, self.z)
    }
}

#[derive(Clone)]
struct Cell {
    cell_type: usize,
    volume: f32,
}

impl Cell {
    fn empty() -> Self {
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

#[derive(Debug)]
struct Data {
    size: V3I,
    cells: Vec<Cell>,
}

impl Clone for Data {
    fn clone(&self) -> Self {
        Data {
            size: self.size,
            cells: self.cells.clone()
        }
    }
}

impl Data {
    fn create(world: V3I) -> Self {
        let data = vec![Cell::empty(); (world.x * world.y * world.z) as usize];
        Data { size: world, cells: data }
    }

    fn get(&self, x: i32, y: i32, z: i32) -> Option<&Cell> {
        let size = &self.size;

        if x < size.x && y < size.y && z < size.z && x >= 0 && y >= 0 && z >= 0 {
            return Some(self.get_unsafe(x, y, z));
        } else {
            return None;
        }
    }

    fn get_relative(&self, x: i32, y: i32, z: i32, delta: V3I) -> Option<&Cell> {
        self.get(x + delta.x, y + delta.y, z + delta.z)
    }

    fn get_unsafe(&self, x: i32, y: i32, z: i32) -> &Cell {
        let size = &self.size;

        return &self.cells[(x*size.y*size.z + y*size.z + z) as usize];
    }

    fn set(&mut self, x: i32, y: i32, z: i32, value: Cell) {
        let size = &self.size;
        self.cells[(x*size.y*size.z + y*size.z + z) as usize] = value;
    }

    fn update_relative(&mut self, x: i32, y: i32, z: i32, delta: V3I, change: f32) {
        let mut current = self.get_unsafe(x + delta.x, y + delta.y, z + delta.z).clone();
        current.volume += change;
        self.set(x + delta.x, y + delta.y, z + delta.z, current);
    }

    fn update<F>(&mut self, x: i32, y: i32, z: i32, f: F)
        where F: Fn(&mut Cell) {

        let size = &self.size;

        let ref mut current = &mut self.cells[(x*size.y*size.z + y*size.z + z) as usize];

        f(current);
    }
}

const H_NEIGHBOURS: [V3I; 4] = [
    V3I { x: -1, y: 0, z: 0},
    V3I { x: 1, y: 0, z: 0},
    V3I { x: 0, y: 0, z: -1},
    V3I { x: 0, y: 0, z: 1},
];

const NANOS_PER_SECOND: u64 = 1000000000;

fn test_update(c: &mut Cell) {
    c.volume += 0.3;
}

fn main() {
    //let world = V3I { x: 100, y: 50, z: 100};
    let world = V3I { x: 2, y: 1, z: 2};
    let wx = world.x;
    let wy = world.y;
    let wz = world.z;
    let mut data = Data::create(world);

    println!("Grid size: {:?}x{:?}x{:?}", wx, wy, wz);
    println!("Grid mem size: {:?} Mb", (size_of::<Cell>() as i32 * wx * wy * wz) as f32 / 1000.0 / 1000.0);
    // Init World
    for x in 0..wx {
        for y in 0..wy {
            for z in 0..wz {
                let mut new_cell = Cell::empty();
                new_cell.volume = x as f32;
                data.set(x, y, z, new_cell);
            }
        }
    }
    let mut frame_start = Instant::now();
    let mut timer = Instant::now();
    let iterations = 100;
    for _i in 0..iterations {
        /*
        println!("");
        for y in 0..wy {
            for x in 0..wx {
                for z in 0..wz {
                    print!("{} ", data.get_unsafe(x, y, z));
                }
                println!("");
            }
        }
        */

        // Move to a non-mutable binding to enforce that it isn't changed.
        let old_data = data;
        let mut new_data = old_data.clone();

        for x in 0..wx {
            for y in 0..wy {
                for z in 0..wz {
                    let cell = old_data.get_unsafe(x, y, z);

                    let neighbours = H_NEIGHBOURS.iter()
                        .map(|delta| { old_data.get_relative(x, y, z, *delta) });

                    let mut sum = cell.volume;
                    let mut total = 1.0;
                    for n in neighbours.into_iter().filter_map(|x| { x }) {
                        sum += n.volume;
                        total += 1.0;
                    }
                    let target_volume = sum / total;
                    //println!("neighbours: {:?}", neighbours);

                    let mut remaining = cell.volume;

                    for delta in H_NEIGHBOURS.iter() {
                        let nx = x + delta.x;
                        let ny = y + delta.y;
                        let nz = z + delta.z;

                        match old_data.get(nx, ny, nz) {
                            Some(n) => {
                                // 0.3, 0.4, 0.3
                                //println!("{}, {}, {}", n, target_volume, remaining);
                                let flow = (target_volume - n.volume).max(0.0).min(remaining);
                                //println!("flow to {:?}: {:?}", delta, flow);

                                new_data.update(x+delta.x, y+delta.y, z+delta.y, |current| current.volume += flow);

                                remaining -= flow;
                            },
                            _ => {}
                        }
                    }
                    // Should be an update?
                    //println!("updating cell: {:?}", remaining - cell);
                    //new_data.update_relative(x, y, z, V3I::zero(), remaining - cell.volume);
                    new_data.update(x, y, z, |current| current.volume += remaining - cell.volume);
                }
            }
        }

        data = new_data;

        /*
        let duration = frame_start.elapsed();
        let fps = NANOS_PER_SECOND / ((duration.as_secs() * NANOS_PER_SECOND) + duration.subsec_nanos() as u64);
        println!("{:?}", fps);
        frame_start = Instant::now();
        */
    }
    let duration = timer.elapsed();
    let fps = NANOS_PER_SECOND / (((duration.as_secs() * NANOS_PER_SECOND) + duration.subsec_nanos() as u64) / iterations);
    println!("Avg. FPS: {:?}", fps);

    println!("");
    for y in 0..wy {
        for x in 0..wx {
            for z in 0..wz {
                print!("{:?} ", data.get_unsafe(x, y, z));
            }
            println!("");
        }
    }
}
