extern crate collaborate;
extern crate gl_winit;
extern crate image;
extern crate polygon;
extern crate structopt;
#[macro_use]
extern crate structopt_derive;
extern crate winit;

use gl_winit::CreateContext;
use polygon::*;
use polygon::anchor::*;
use polygon::camera::*;
use polygon::gl::GlRender;
use polygon::light::*;
use polygon::math::*;
use polygon::mesh_instance::*;
use std::path::PathBuf;
use std::time::*;
use structopt::StructOpt;
use winit::*;

mod collada;

#[derive(Debug, StructOpt)]
#[structopt(name = "polyview", about = "A mesh viewer for the Polygon rendering engine.")]
struct CliArgs {
    #[structopt(help = "The path to the mesh to be viewed")]
    path: String,
}

fn main() {
    let args = CliArgs::from_args();

    // Build a triangle mesh.
    let mesh = collada::load_mesh(args.path).unwrap();

    // Open a window.
    let mut events_loop = EventsLoop::new();
    let window = WindowBuilder::new()
        .with_dimensions(800, 800)
        .build(&events_loop)
        .expect("Failed to open window");

    // Create the OpenGL context and the renderer.
    let context = window.create_context().expect("Failed to create GL context");
    let mut renderer = GlRender::new(context).expect("Failed to create GL renderer");

    // Send the mesh to the GPU.
    let gpu_mesh = renderer.register_mesh(&mesh);

    // Create an anchor and register it with the renderer.
    let mut anchor = Anchor::new();
    anchor.set_position(Point::new(0.0, 0.0, 0.0));
    let mesh_anchor_id = renderer.register_anchor(anchor);

    let mut material = renderer.default_material();
    material.set_color("surface_color", Color::rgb(1.0, 0.0, 0.0));
    material.set_color("surface_specular", Color::rgb(1.0, 1.0, 1.0));
    material.set_f32("surface_shininess", 4.0);

    // Create a mesh instance, attach it to the anchor, and register it with the renderer.
    let mut mesh_instance = MeshInstance::with_owned_material(gpu_mesh, material);
    mesh_instance.set_anchor(mesh_anchor_id);
    renderer.register_mesh_instance(mesh_instance);

    // Create a camera and an anchor for it.
    let mut camera_anchor = Anchor::new();
    camera_anchor.set_position(Point::new(0.0, 0.0, 10.0));
    let camera_anchor_id = renderer.register_anchor(camera_anchor);

    // Create the light and an anchor for it.
    let light = Light::directional(Vector3::new(1.0, -1.0, -1.0), 0.25, Color::rgb(1.0, 1.0, 1.0));
    renderer.register_light(light);

    let mut camera = Camera::default();
    camera.set_anchor(camera_anchor_id);
    renderer.register_camera(camera);

    let mut loop_active = true;
    let frame_time = Duration::from_secs(1) / 60;
    let mut next_loop_time = Instant::now() + frame_time;
    while loop_active {
        events_loop.poll_events(|event| {
            match event {
                Event::WindowEvent { event: WindowEvent::Closed, .. } => {
                    loop_active = false;
                }

                _ => {}
            }
        });
        if !loop_active { break; }

        {
            let mesh_anchor = renderer.get_anchor_mut(mesh_anchor_id).unwrap();
            let orientation = mesh_anchor.orientation();
            let change = Orientation::from_eulers(TAU / 4.0 / 60.0, TAU / 6.0 / 60.0, TAU / 8.0 / 60.0);
            mesh_anchor.set_orientation(orientation + change);
        }

        // Render the mesh.
        renderer.draw();

        // Wait for the next frame.
        // TODO: Wait more efficiently by sleeping the thread.
        while Instant::now() < next_loop_time {}
        next_loop_time += frame_time;
    }
}
