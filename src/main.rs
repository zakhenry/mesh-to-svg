extern crate nalgebra as na;

#[macro_use]
extern crate approx; // For the macro relative_eq!

use serde::{Deserialize, Serialize};
use serde_json;

extern crate clap;
use clap::{App, Arg};

mod lines;
mod mesh;
mod scene;
mod svg_renderer;
use mesh_to_svg::scene::{Scene};
use mesh_to_svg::mesh::{Mesh, Wireframe};
use mesh_to_svg::find_categorized_line_segments;
use mesh_to_svg::svg_renderer::{SvgConfig, screen_space_lines_to_fitted_svg};
use std::{fmt, io};
use std::fs::File;
use std::io::BufReader;
use wasm_bindgen::__rt::core::fmt::{Error, Formatter};

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

#[derive(Serialize, Deserialize)]
struct JsonMesh {
    id: String,
    mesh: MeshData,
    edgesMesh: WireframeData,
}

#[derive(Serialize, Deserialize)]
enum ParserError {
    Unknown,
}

impl fmt::Debug for ParserError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            ParserError::Unknown => write!(f, "Unknown error!"),
        }
    }
}

impl JsonMesh {
    pub fn to_mesh(&self) -> Result<(Mesh, Wireframe), ParserError> {
        Ok((
            Mesh::new(
                self.mesh.indices.to_owned(),
                self.mesh.positions.to_owned(),
                self.mesh.normals.to_owned(),
            ),
            Wireframe::new(self.mesh.indices.to_owned(), self.mesh.positions.to_owned()),
        ))
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
                .long("file")
                .help("Set file to parse")
                .required(true),
        )
        .get_matches();

    let file_path = arg_matches
        .value_of("file")
        .expect("You must set a file argument!");

    // print!("using input file {}", file_path);

    let file = File::open(file_path).expect("Could not open file");
    let reader = BufReader::new(file);

    let mesh_json: JsonMesh =
        serde_json::from_reader(reader).expect("Could not parse JSON mesh file");

    let (mesh, wireframe) = mesh_json.to_mesh().unwrap();

    // print!("Mesh parsed. Index count: {}", mesh.indices.len());

    let scene = Scene::new_test();

    let svg_config = SvgConfig::new_default(scene.width as i32, scene.height as i32);

    let segments = find_categorized_line_segments(&mesh, &wireframe, &scene);

    let svg = screen_space_lines_to_fitted_svg(&segments, &svg_config);

    print!("{}", svg);

}
