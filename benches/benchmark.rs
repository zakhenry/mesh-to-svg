use std::fs::File;
use std::io::BufReader;

use criterion::{black_box, criterion_group, criterion_main, Criterion};
use serde::{Deserialize, Serialize};

use mesh_to_svg::lines::split_lines_by_intersection;
use mesh_to_svg::mesh::{Mesh, Wireframe};
use mesh_to_svg::scene::Scene;

// @todo these structs are duplicated from examples/bin, there should be a way to share them
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
    edgesMesh: WireframeData,
}

impl JsonMesh {
    pub fn to_mesh(&self) -> (Mesh, Wireframe) {
        (
            Mesh::new(
                self.mesh.indices.to_owned(),
                self.mesh.positions.to_owned(),
                self.mesh.normals.to_owned(),
            ),
            Wireframe::new(
                self.edgesMesh.indices.to_owned(),
                self.edgesMesh.positions.to_owned(),
            ),
        )
    }
}

fn get_deps(mesh_name: &str) -> (Mesh, Wireframe, Scene) {
    let file_path = format!("meshes/{}.json", mesh_name);

    let file = File::open(file_path).expect("Could not open file");
    let reader = BufReader::new(file);

    let mesh_json: JsonMesh =
        serde_json::from_reader(reader).expect("Could not parse JSON mesh file");

    let (mesh, wireframe) = mesh_json.to_mesh();

    let scene = Scene::new_test();

    (mesh, wireframe, scene)
}

pub fn criterion_benchmark(_: &mut Criterion) {
    let mut c = Criterion::default().sample_size(10);

    let (mesh, wireframe, scene) = get_deps("raspi");

    let mut edges = mesh.find_edge_lines(&scene, false);
    edges.append(&mut wireframe.edges());
    let projected = scene.project_lines(&edges);

    c.bench_function("split_lines_by_intersection(raspi)", |b| {
        b.iter(|| {
            split_lines_by_intersection(black_box(&projected));
        })
    });

    // let mut group = c.benchmark_group("split_lines_by_intersection");
    //
    // for capacity in 0..10usize {
    //     group.bench_with_input(BenchmarkId::from_parameter(capacity), &capacity, |b, &capacity| {
    //
    //         b.iter(|| {
    //             split_lines_by_intersection(black_box(&projected), black_box(capacity));
    //         })
    //     });
    // }
    // group.finish();

    // let split_lines = split_lines_by_intersection(&projected);
    //
    // c.bench_function("get_visibility(raspi)", |b| {
    //     b.iter(|| {
    //         partition_visibility(black_box(&mesh), black_box(&scene), black_box(&split_lines))
    //     })
    // });
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
