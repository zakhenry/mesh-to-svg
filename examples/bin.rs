extern crate clap;
extern crate ctrlc;
extern crate drawille;
extern crate log_update;
extern crate term_size;

use std::f32::consts::PI;
use std::ffi::OsStr;
use std::fs::File;
use std::io::stdout;
use std::io::BufReader;
use std::path::Path;
use std::process;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread::sleep;
use std::time::{Duration, Instant};

use clap::{App, Arg, ArgMatches, SubCommand};
use drawille::Canvas;
use log_update::LogUpdate;
use nalgebra::{Matrix4, Rotation3, Vector3};
use serde::{Deserialize, Serialize};
use serde_json;

use mesh_to_svg::find_categorized_line_segments;
use mesh_to_svg::lines::{LineSegmentCategorized, LineVisibility};
use mesh_to_svg::mesh::{Mesh, Wireframe};
use mesh_to_svg::scene::Scene;
use mesh_to_svg::svg_renderer::{
    scale_screen_space_lines, screen_space_lines_to_fitted_svg, SvgConfig,
};

#[derive(Serialize, Deserialize)]
struct MeshData {
    indices: Option<Vec<usize>>,
    positions: Vec<f32>,
    normals: Vec<f32>,
}

#[derive(Serialize, Deserialize)]
struct WireframeData {
    indices: Option<Vec<usize>>,
    positions: Vec<f32>,
}

#[allow(non_snake_case)]
#[derive(Serialize, Deserialize)]
struct JsonMesh {
    id: String,
    mesh: MeshData,
    edgesMesh: Option<WireframeData>,
}

trait Meshable {
    fn to_mesh(&self) -> (Mesh, Option<Wireframe>);
}

impl Meshable for JsonMesh {
    fn to_mesh(&self) -> (Mesh, Option<Wireframe>) {
        (
            Mesh::new(
                self.mesh.indices.to_owned(),
                self.mesh.positions.to_owned(),
                self.mesh.normals.to_owned(),
            ),
            self.edgesMesh
                .as_ref()
                .map(|edges| Wireframe::new(edges.indices.to_owned(), edges.positions.to_owned())),
        )
    }
}

impl Meshable for stl_io::IndexedMesh {
    // @todo - due to the Mesh type needing vertex normals, we have to duplicate facet normals, however
    // this approach is duplicating vertices too
    fn to_mesh(&self) -> (Mesh, Option<Wireframe>) {
        let mut indices = vec![0; self.faces.len() * 3];
        let mut vertices = vec![0.0; indices.len() * 3];
        let mut normals = vec![0.0; indices.len() * 3];

        for (triangle_index, triangle) in self.faces.iter().enumerate() {
            for (index, &vertex_index) in triangle.vertices.iter().enumerate() {
                let current_index = triangle_index * 3 + index;

                indices[current_index] = current_index;

                let vertex = self.vertices[vertex_index];

                for i in 0..3 {
                    vertices[current_index * 3 + i] = vertex[i];
                    normals[current_index * 3 + i] = triangle.normal[i];
                }
            }
        }

        let mesh = Mesh::new(Some(indices), vertices, normals);

        (mesh, None)
    }
}

fn main() {
    let arg_matches = App::new("mesh-to-svg")
        .version("0.0.0")
        .author("Zak Henry @zak")
        .about("Convert mesh to svg line drawing")
        .arg(
            Arg::with_name("file")
                .takes_value(true)
                .short("f")
                .long("file")
                .help("Set file to parse")
                .required(true),
        )
        .subcommand(
            SubCommand::with_name("term")
                .about("output mesh to terminal")
                .arg(
                    Arg::with_name("output_width")
                        .takes_value(true)
                        .short("w")
                        .long("output-width")
                        .help("Output width [defaults to term width]"),
                )
                .arg(
                    Arg::with_name("output_height")
                        .takes_value(true)
                        .short("h")
                        .long("output-height")
                        .help(
                            "Output height [defaults to maintain aspect ratio with scene source]",
                        ),
                )
                .arg(
                    Arg::with_name("animate")
                        .long("animate")
                        .help("Animate the terminal"),
                ),
        )
        .get_matches();

    let file_path = arg_matches
        .value_of("file")
        .expect("You must set a file argument!");

    let (mesh, wireframe) = match get_extension_from_filename(&file_path) {
        Some("json") => {
            let file = File::open(file_path).expect("Could not open file");
            let reader = BufReader::new(file);
            let mesh_json: JsonMesh =
                serde_json::from_reader(reader).expect("Could not parse JSON mesh file");

            mesh_json.to_mesh()
        }
        Some("stl") => {
            let mut file = File::open(file_path).expect("Could not open file");
            let stl = stl_io::read_stl(&mut file).unwrap();

            stl.to_mesh()
        }
        Some(_) | None => panic!("Unsupported file extension"),
    };

    let scene = Scene::new_test();

    let svg_config = SvgConfig::new_default(scene.width as i32, scene.height as i32);

    let segments = find_categorized_line_segments(&mesh, &wireframe, &scene);

    if let Some(term_subcommand) = arg_matches.subcommand_matches("term") {
        if term_subcommand.is_present("animate") {
            animate(&mesh, &wireframe, &term_subcommand);
        } else {
            let terminal_drawing = draw_terminal(segments, &scene, &term_subcommand);
            println!("{}", terminal_drawing);
        }
    } else {
        let svg = screen_space_lines_to_fitted_svg(&segments, &svg_config);
        println!("{}", svg);
    }
}

fn get_extension_from_filename(filename: &str) -> Option<&str> {
    Path::new(filename).extension().and_then(OsStr::to_str)
}

fn draw_terminal(
    segments: Vec<LineSegmentCategorized>,
    scene: &Scene,
    matches: &ArgMatches,
) -> String {
    let (w, _) = term_size::dimensions().unwrap_or((100, 0));

    // not entirely sure why the adjustment is needed, the canvas is 2x term as the braille char are
    // at double density, but the `- 1` is inexplicable
    let term_canvas_width: i32 = w as i32 * 2 - 1;

    let width: i32 = match matches.value_of("output_width") {
        Some(w) => w.parse::<i32>().expect("output_height must be a number!"),
        None => term_canvas_width as i32,
    };

    let height: i32 = match matches.value_of("output_height") {
        Some(h) => h.parse::<i32>().expect("output_height must be a number!"),
        None => (scene.height / scene.width * width as f32) as i32,
    };

    let svg_config = SvgConfig::new(
        scene.width as i32,
        scene.height as i32,
        Some(width),
        Some(height),
        Some(5),
        None,
        None,
        None,
        None,
        None,
        Some(false),
    );

    let lines: Vec<LineSegmentCategorized> = scale_screen_space_lines(&segments, &svg_config)
        .into_iter()
        .filter(|line| match line.visibility {
            LineVisibility::VISIBLE => true,
            LineVisibility::OBSCURED => false,
        })
        .collect();

    let mut canvas = Canvas::new(svg_config.width as u32, svg_config.height as u32);

    for line in lines {
        canvas.line(
            line.line_segment.from.x as u32,
            line.line_segment.from.y as u32,
            line.line_segment.to.x as u32,
            line.line_segment.to.y as u32,
        );
    }

    canvas.frame()
}

fn animate(mesh: &Mesh, wireframe: &Option<Wireframe>, matches: &ArgMatches) {
    let mut log_update = LogUpdate::new(stdout()).unwrap();

    let running = Arc::new(AtomicBool::new(true));

    let r = running.clone();

    ctrlc::set_handler(move || {
        r.store(false, Ordering::SeqCst);
    })
    .expect("Error setting Ctrl-C handler");

    let mut scene_angles: Vec<String> = vec![];

    let count = 50;

    let up = Vector3::z_axis();
    let angle = (PI * 2.0) / count as f32;
    let rotation: Matrix4<f32> = Rotation3::from_axis_angle(&up, angle).to_homogeneous();

    let mut scene = Scene::new_test();
    for i in 0..count {
        scene.mesh_world_matrix *= &rotation;

        let start = Instant::now();
        let segments = find_categorized_line_segments(&mesh, &wireframe, &scene);
        let terminal_drawing = draw_terminal(segments, &scene, &matches);
        let duration = start.elapsed();

        let progress = format!(
            "Rendered {} of {} angles ({:?})\n\n{}",
            i, count, duration, &terminal_drawing
        );

        log_update.render(&format!("{}", progress)).unwrap();
        scene_angles.push(terminal_drawing);

        if !running.load(Ordering::SeqCst) {
            &log_update.done().unwrap(); // done will print the cursor unhiding control char
            process::exit(0);
        }
    }

    loop {
        for drawing in &scene_angles {
            log_update.render(&format!("{}", drawing)).unwrap();
            sleep(Duration::from_millis(200));

            if !running.load(Ordering::SeqCst) {
                log_update.done().unwrap(); // done will print the cursor unhiding control char
                process::exit(0);
            }
        }
    }
}
