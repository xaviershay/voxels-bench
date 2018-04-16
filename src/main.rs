extern crate rand;
#[macro_use]
extern crate gfx;
extern crate piston_window;
extern crate shader_version;

use std::mem::size_of;
use std::time::{Duration, Instant};
use std::sync::{Arc,RwLock,mpsc};
use std::thread;
use rand::{Rng,thread_rng};

mod types;

use types::*;

gfx_defines!{
    vertex Vertex {
        a_pos: [i8; 4] = "a_pos",
        a_tex_coord: [i8; 2] = "a_tex_coord",
    }

    pipeline pipe {
        vbuf: gfx::VertexBuffer<Vertex> = (),
        u_model_view_proj: gfx::Global<[[f32; 4]; 4]> = "u_model_view_proj",
        //t_color: gfx::TextureSampler<[f32; 4]> = "t_color",
        out_color: gfx::RenderTarget<::gfx::format::Srgba8> = "FragColor",
        out_depth: gfx::DepthTarget<::gfx::format::DepthStencil> =
            gfx::preset::depth::LESS_EQUAL_WRITE,
    }
}

impl Vertex {
    fn new(pos: [i8; 3], tc: [i8; 2]) -> Vertex {
        Vertex {
            a_pos: [pos[0], pos[1], pos[2], 1],
            a_tex_coord: tc,
        }
    }
}

fn main() {
    use piston_window::*;
    use gfx::traits::*;
    use shader_version::Shaders;
    use shader_version::glsl::GLSL;
    use gfx::{Primitive,ShaderSet};
    use gfx::state::Rasterizer;

    let opengl = OpenGL::V3_3;

    let mut window: PistonWindow =
        WindowSettings::new("piston: cube", [640, 480])
        .exit_on_esc(true)
        .samples(4)
        .opengl(opengl)
        .build()
        .unwrap();

    let ref mut factory = window.factory.clone();

    let vertex_data = vec![
        Vertex::new([-1, -1, 1], [0, 0]),
        Vertex::new([1, -1, 1], [1, 0]),
        Vertex::new([1, 1, 1], [1, 1]),
        Vertex::new([-1, 1, 1], [0, 1]),
    ];
    let index_data: &[u16] = &[
        0, 1, 2, 2, 3, 0,
    ];
    let (vbuf, slice) = factory.create_vertex_buffer_with_slice(&vertex_data, index_data);

    let vertex_shader = r#"
    #version 330 core
    layout (location = 0) in ivec3 a_pos;

    void main()
    {
        gl_Position = vec4(a_pos.x*0.3, a_pos.y*0.3, a_pos.z*0.3, 1.0);
    }
    "#;
    let fragment_shader = r#"
    #version 330 core
    out vec4 FragColor;

    void main()
    {
        FragColor = vec4(1.0f, 0.5f, 0.2f, 1.0f);
    } 
    "#;
    let vs = factory.create_shader_vertex(vertex_shader.as_bytes()).expect("Failed to compile vertex shader");
    let fs = factory.create_shader_pixel(fragment_shader.as_bytes()).expect("Failed to compile fragment shader");
    let ss = ShaderSet::Simple(vs, fs);

    //let glsl = opengl.to_glsl();
    let pso = factory.create_pipeline_state(
        &ss,
        Primitive::TriangleList,
        Rasterizer::new_fill(),
        pipe::new()
    ).unwrap();

    let mut data = pipe::Data {
            vbuf: vbuf.clone(),
            u_model_view_proj: [[0.0; 4]; 4],
            out_color: window.output_color.clone(),
            out_depth: window.output_stencil.clone(),
    };

    while let Some(e) = window.next() {
        window.draw_3d(&e, |window| {
            let args = e.render_args().unwrap();

            window.encoder.clear(&window.output_color, [0.3, 0.3, 0.3, 1.0]);
            window.encoder.clear_depth(&window.output_stencil, 1.0);

            window.encoder.draw(&slice, &pso, &data);
        });

        if let Some(_) = e.resize_args() {
            data.out_color = window.output_color.clone();
            data.out_depth = window.output_stencil.clone();
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

fn main2() {
    //let world = V3I { x: 100, y: 100, z: 100};
    let world = V3I { x: 20, y: 1, z: 20};
    //let world = V3I { x: 20, y: 1, z: 20};
    let wx = world.x;
    let wy = world.y;
    let wz = world.z;
    let data = Arc::new(RwLock::new(World::create(world)));

    let locations =
        (0..wx).flat_map(|x| {
            (0..wy).flat_map(move |y| {
                (0..wz).map (move |z| {
                    V3I::create(x, y, z)
                })
            })
        }).collect::<Vec<_>>();

    // Init World
    {
        let mut value = data.write().unwrap();
        let mut rng = thread_rng();

        for &location in locations.iter() {
            value.update(location, |c| {
                let a: f32 = rng.gen();
                c.volume = if a < 0.5 {
                    1.0
                } else {
                    0.0
                }
            });
        }
    }

    let physics_data = Arc::clone(&data);
    let (physics_tx, physics_rx) = mpsc::channel();
    let physics_thread = thread::spawn(move || {
        let max_flow_per_sec = 1.0;
        let mut delta_time = 0.0;

        loop {
            match physics_rx.try_recv() {
                Ok(_) => break,
                _ => {},
            }

            let max_flow = if delta_time > 0.0 {
                max_flow_per_sec * delta_time
            } else {
                // Default max flow of 30FPS
                max_flow_per_sec * (1.0 / 30.0)
            };

            let frame_start = Instant::now();

            // Use a scope here to force releasing the read lock before we try to obtain the write
            // lock in the next section.
            let new_data = {
                let old_data = physics_data.read().unwrap();
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
                            let flow = (target_volume - n.volume).max(0.0).min(remaining).min(max_flow);

                            new_data.update(nl, |current| current.volume += flow);

                            remaining -= flow;
                        }
                    }
                    new_data.update(location, |current| current.volume += remaining - cell.volume);
                }
                let duration = frame_start.elapsed();
                delta_time = duration.as_secs() as f32 + (duration.subsec_nanos() as f32 / NANOS_PER_SECOND as f32);
                let fps = 1.0 / delta_time;
                new_data.physics_fps = fps;
                new_data
            };

            {
                let mut d = physics_data.write().unwrap();
                *d = new_data;
            }
        }
    });

    loop {
        thread::sleep(Duration::from_millis(300));
        let d = data.read().unwrap();
        print!("{}[2J", 27 as char);
        for x in 0..wx {
            for z in 0..wz {
                let location = V3I::create(x, wy-1, z);

                print!("{: >6.2} ", d.get_unsafe(location).volume);
            }
            println!("");
        }
        println!("");
        println!("(Only showing top layer)");
        println!("");
        println!("Grid size: {:?}x{:?}x{:?}", wx, wy, wz);
        println!("Grid mem size: {:?} Mb",
             (size_of::<Cell>() as i32 * wx * wy * wz) as f32 / 1000.0 / 1000.0);
        println!("Physics FPS: {}", d.physics_fps);
    }
    physics_tx.send(true).unwrap();
    physics_thread.join().unwrap();
}
