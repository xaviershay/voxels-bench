use std::fmt;
use std::ops::Add;
use std::mem::size_of;
use std::time::{Instant};

#[derive(Clone, Copy)]
struct V3I {
    x: i32,
    y: i32,
    z: i32,
}

impl V3I {
    fn create(x: i32, y: i32, z: i32) -> Self {
        V3I { x: x, y: y, z: z }
    }
    fn zero() -> Self {
        V3I::create(0, 0, 0)
    }
}

impl Add for V3I {
    type Output = V3I;

    fn add(self, other: V3I) -> V3I {
        V3I::create(self.x + other.x, self.y + other.y, self.z + other.z)
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

    fn get(&self, v: V3I) -> Option<&Cell> {
        let size = &self.size;

        if v.x < size.x && v.y < size.y && v.z < size.z && v.x >= 0 && v.y >= 0 && v.z >= 0 {
            return Some(self.get_unsafe(v));
        } else {
            return None;
        }
    }

    fn get_unsafe(&self, v: V3I) -> &Cell {
        let size = &self.size;

        return &self.cells[(v.x*size.y*size.z + v.y*size.z + v.z) as usize];
    }

    fn update<F>(&mut self, v: V3I, f: F)
        where F: Fn(&mut Cell) {

        let size = &self.size;

        if v.x < size.x && v.y < size.y && v.z < size.z && v.x >= 0 && v.y >= 0 && v.z >= 0 {
            let ref mut current = &mut self.cells[(v.x*size.y*size.z + v.y*size.z + v.z) as usize];

            f(current);
        }
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
    let world = V3I { x: 100, y: 50, z: 100};
    //let world = V3I { x: 2, y: 1, z: 2};
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
                let location = V3I::create(x, y, z);
                data.update(location, |c| c.volume = x as f32);
            }
        }
    }
    let mut frame_start = Instant::now();
    let mut timer = Instant::now();
    let iterations = 50;
    for _i in 0..iterations {
        /*
        println!("");
        for y in 0..wy {
            for x in 0..wx {
                for z in -1..wz {
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
                    let location = V3I::create(x, y, z);

                    let cell = old_data.get_unsafe(location);

                    let mut sum = cell.volume;
                    let mut total = 1.0;

                    for delta in H_NEIGHBOURS.iter() {
                        let nl = location + *delta;

                        for n in old_data.get(nl) {
                            sum += n.volume;
                            total += 1.0;
                        }
                    }

                    let target_volume = sum / total;

                    let mut remaining = cell.volume;

                    // Doing a second loop here, with the "double fetching" of (nl, n) is actually
                    // faster than precomputing. Probably because pre-computing ends up allocating
                    // extra memory [citation needed]. Given that H_NEIGHBOURS is a fixed-size
                    // array, there's likely a way to make it work without extra allocations...
                    for delta in H_NEIGHBOURS.iter() {
                        let nl = location + *delta;

                        for n in old_data.get(nl) {
                            let flow = (target_volume - n.volume).max(0.0).min(remaining);

                            new_data.update(nl, |current| current.volume += flow);

                            remaining -= flow;
                        }
                    }
                    new_data.update(location, |current| current.volume += remaining - cell.volume);
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

    /*
    println!("");
    for y in 0..wy {
        for x in 0..wx {
            for z in 0..wz {
                print!("{:?} ", data.get_unsafe(x, y, z));
            }
            println!("");
        }
    }
    */
}
