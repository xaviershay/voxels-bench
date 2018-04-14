use std::mem::size_of;
use std::time::{Instant};
use std::sync::{Arc,RwLock};

mod types;

use types::*;

const H_NEIGHBOURS: [V3I; 4] = [
    V3I { x: -1, y: 0, z: 0},
    V3I { x: 1, y: 0, z: 0},
    V3I { x: 0, y: 0, z: -1},
    V3I { x: 0, y: 0, z: 1},
];

const NANOS_PER_SECOND: u64 = 1000000000;

fn main() {
    let world = V3I { x: 100, y: 50, z: 100};
    //let world = V3I { x: 2, y: 1, z: 2};
    let wx = world.x;
    let wy = world.y;
    let wz = world.z;
    let data = RwLock::new(World::create(world));

    let locations =
        (0..wx).flat_map(|x| {
            (0..wy).flat_map(move |y| {
                (0..wz).map (move |z| {
                    V3I::create(x, y, z)
                })
            })
        }).collect::<Vec<_>>();

    println!("Grid size: {:?}x{:?}x{:?}", wx, wy, wz);
    println!("Grid mem size: {:?} Mb", (size_of::<Cell>() as i32 * wx * wy * wz) as f32 / 1000.0 / 1000.0);
    // Init World
    {
        let mut value = data.write().unwrap();

        for &location in locations.iter() {
            value.update(location, |c| c.volume = location.x as f32);
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

        // Use a scope here to force releasing the read lock before we try to obtain the write lock
        // in the next section.
        let new_data = {
            let old_data = data.read().unwrap();
            let mut new_data = old_data.clone();

            for &location in locations.iter() {
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
            new_data
        };

        {
            let mut d = data.write().unwrap();
            *d = new_data;
        }

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
                let location = V3I::create(x, y, z);

                print!("{:?} ", data.get_unsafe(location));
            }
            println!("");
        }
    }
    */
}
