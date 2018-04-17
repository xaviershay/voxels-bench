#![feature(fs_read_write)]

extern crate rand;
#[macro_use]
extern crate gfx;
extern crate piston_window;
extern crate shader_version;
extern crate vecmath;
extern crate camera_controllers;

use std::time::{Instant};
use std::sync::{Arc,RwLock,mpsc};
use std::thread;
use std::fs;
use rand::{Rng,thread_rng};

use piston_window::*;
use gfx::traits::*;
use gfx::{Primitive,ShaderSet};
use gfx::state::Rasterizer;
use gfx::Slice;
use camera_controllers::{
    FirstPersonSettings,
    FirstPerson,
    CameraPerspective,
    model_view_projection
};

mod types;

use types::*;

const H_NEIGHBOURS: [V3I; 4] = [
    V3I { x: -1, y: 0, z: 0},
    V3I { x: 1, y: 0, z: 0},
    V3I { x: 0, y: 0, z: -1},
    V3I { x: 0, y: 0, z: 1},
];

const NANOS_PER_SECOND: u64 = 1000000000;

gfx_defines!{
    vertex Vertex {
        a_pos: [i8; 4] = "a_pos",
        a_tex_coord: [i8; 2] = "a_tex_coord",
    }

    pipeline pipe {
        vbuf: gfx::VertexBuffer<Vertex> = (),
        u_model_view_proj: gfx::Global<[[f32; 4]; 4]> = "u_model_view_proj",
        t_data: gfx::TextureSampler<[f32; 4]> = "t_data",
        world_size: gfx::Global<[i32; 3]> = "world_size",
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

fn create_shader_set<R: gfx::Resources, D: Factory<R>>(factory: &mut D) -> ShaderSet<R> {
    let cwd = std::env::current_dir().unwrap();

    let vertex_shader = fs::read(cwd.join("assets/voxels.glslv")).unwrap();
    let geometry_shader = fs::read(cwd.join("assets/voxels.glslg")).unwrap();
    let fragment_shader = fs::read(cwd.join("assets/voxels.glslf")).unwrap();

    let vs = factory.create_shader_vertex(&vertex_shader).expect("Failed to compile vertex shader");
    let gs = factory.create_shader_geometry(&geometry_shader).expect("Failed to compile geometry shader");
    let fs = factory.create_shader_pixel(&fragment_shader).expect("Failed to compile fragment shader");

    ShaderSet::Geometry(vs, gs, fs)
}
fn main() {

    let world = V3I { x: 3, y: 2, z: 3};
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

    let opengl = OpenGL::V3_3;

    let mut window: PistonWindow =
        WindowSettings::new("piston: cube", [640, 480])
        .exit_on_esc(true)
        .samples(4)
        .opengl(opengl)
        .build()
        .unwrap();

    let ref mut factory = window.factory.clone();
    let mut vertex_data = vec![Vertex::new([0,0,0], [0,0]); locations.len()];
    for l in &locations {
        vertex_data.push(Vertex::new([l.x as i8,l.y as i8,l.z as i8], [0,0]));
    }

    let vbuf = factory.create_vertex_buffer(&vertex_data);
    let slice = Slice::new_match_vertex_buffer(&vbuf);

    //let glsl = opengl.to_glsl();
    // Store height in alpha channel of RGBA8. Bit weird, but keeping like this
    // because will want to pass through other data as well, and also not clear
    // how to just include a single float - looks like no matter what the
    // geometry shader is going to want to interpret as RGBA.
    let texels = vec![ [0x00, 0x00, 0x00, 0x11]; locations.len()];

    let (_, texture_view) = factory.create_texture_immutable::<gfx::format::Rgba8>(
        gfx::texture::Kind::D3(wx as u16, wy as u16, wz as u16),
        gfx::texture::Mipmap::Provided,
&[&texels]).unwrap();

    let sinfo = gfx::texture::SamplerInfo::new(
        gfx::texture::FilterMethod::Scale,
        gfx::texture::WrapMode::Clamp);
    //sinfo.border = PackedColor::from([0.0, 0.0, 0.0, 0.5]);

    let sampler = factory.create_sampler(sinfo);
    let mut gfx_data = pipe::Data {
            vbuf: vbuf.clone(),
            u_model_view_proj: [[0.0; 4]; 4],
            t_data: (texture_view, sampler),
            world_size: [wx, wz, wy],
            out_color: window.output_color.clone(),
            out_depth: window.output_stencil.clone(),
    };

    let mut t: u8 = 0;
    let mut frame_count = 0;
    let mut frame_start = Instant::now();
    //let world = V3I { x: 100, y: 100, z: 100};

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
        let locations =
            (0..wx).flat_map(|x| {
                (0..wy).flat_map(move |y| {
                    (0..wz).map (move |z| {
                        V3I::create(x, y, z)
                    })
                })
        }).collect::<Vec<_>>();
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
                // First iteration won't do anything. We need a delta for correct calculations.
                0.0
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

    let get_projection = |w: &PistonWindow| {
        let draw_size = w.window.draw_size();
        CameraPerspective {
            fov: 90.0, near_clip: 0.1, far_clip: 1000.0,
            aspect_ratio: (draw_size.width as f32) / (draw_size.height as f32)
        }.projection()
    };

    let model = vecmath::mat4_id();
    let mut projection = get_projection(&window);
    let mut first_person = FirstPerson::new(
        [-0.5, 1.5, 4.0],
        FirstPersonSettings::keyboard_wasd()
    );

    //window.set_capture_cursor(true);
    let ss = create_shader_set(factory);

    let mut pso = factory.create_pipeline_state(
        &ss,
        Primitive::PointList,
        Rasterizer::new_fill(),
        pipe::new()
    ).unwrap();

    while let Some(e) = window.next() {
        first_person.event(&e);

        if frame_start.elapsed().as_secs() >= 1 {
            println!("{}", frame_count);
            frame_count = 0;
            frame_start = Instant::now();

            let ss = create_shader_set(factory);

            pso = factory.create_pipeline_state(
                &ss,
                Primitive::PointList,
                Rasterizer::new_fill(),
                pipe::new()
            ).unwrap();
        }
        frame_count += 1;
        if t < 255 {
            t += 1;    
        } else {
            t = 0;
        }
        let mut texels2 = Vec::with_capacity(locations.len());
        let d = data.read().unwrap();
        for location in &locations {
            texels2.push([0, 0, 0, (d.get_unsafe(*location).volume * 255.0) as u8]);
        }
        let sampler = factory.create_sampler(sinfo);
        // TODO: What about a mutable texture? Is that a thing?
        let (_, texture_view) = factory.create_texture_immutable::<gfx::format::Rgba8>(
            gfx::texture::Kind::D3(wx as u16, wz as u16, wy as u16),
            gfx::texture::Mipmap::Provided,
            &[&texels2]).unwrap();
        gfx_data.t_data = (texture_view, sampler);
        window.draw_3d(&e, |window| {
            let args = e.render_args().unwrap();

            window.encoder.clear(&window.output_color, [0.3, 0.3, 0.3, 1.0]);
            window.encoder.clear_depth(&window.output_stencil, 1.0);

            gfx_data.u_model_view_proj = model_view_projection(
                model,
                first_person.camera(args.ext_dt).orthogonal(),
                projection
            );
            window.encoder.draw(&slice, &pso, &gfx_data);
        });

        if let Some(_) = e.resize_args() {
            projection = get_projection(&window);
            gfx_data.out_color = window.output_color.clone();
            gfx_data.out_depth = window.output_stencil.clone();
        }
    }

    physics_tx.send(true).unwrap();
    physics_thread.join().unwrap();
}
