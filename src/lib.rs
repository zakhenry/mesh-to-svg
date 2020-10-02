#[macro_use]
extern crate approx;
extern crate nalgebra as na;
extern crate web_sys;

use wasm_bindgen::prelude::*;

use lines::{get_visibility, split_lines_by_intersection, LineSegmentCategorized};
use mesh::{Mesh, Wireframe};
use scene::{Ray, Scene};
use svg_renderer::{screen_space_lines_to_fitted_svg, SvgConfig};
use utils::set_panic_hook;

use crate::lines::{dedupe_lines, dedupe_lines_faster, ProjectedSplitLine};

#[macro_use]
mod utils;
pub mod lines;
pub mod mesh;
pub mod scene;
pub mod svg_renderer;

use std::time::Instant;

// For the macro relative_eq!

#[wasm_bindgen]
pub fn mesh_to_svg_lines(
    canvas_width: i32,
    canvas_height: i32,
    mesh_indices: Box<[usize]>,
    mesh_vertices: Box<[f32]>,
    mesh_normals: Box<[f32]>,
    wireframe_indices: Option<Box<[usize]>>,
    wireframe_vertices: Option<Box<[f32]>>,
    view_matrix: Box<[f32]>,
    projection_matrix: Box<[f32]>,
    mesh_world_matrix: Box<[f32]>,
    svg_config_width: Option<i32>,
    svg_config_height: Option<i32>,
    svg_config_margin: Option<i32>,
    svg_config_visible_stroke_width: Option<i32>,
    svg_config_visible_stroke: Option<String>,
    svg_config_hide_obscured: Option<bool>,
    svg_config_obscured_stroke_width: Option<i32>,
    svg_config_obscured_stroke: Option<String>,
    svg_config_fit_lines: Option<bool>,
) -> String {
    set_panic_hook();

    let svg_config = SvgConfig::new(
        canvas_width,
        canvas_height,
        svg_config_width,
        svg_config_height,
        svg_config_margin,
        svg_config_visible_stroke_width,
        svg_config_visible_stroke,
        svg_config_hide_obscured,
        svg_config_obscured_stroke_width,
        svg_config_obscured_stroke,
        svg_config_fit_lines,
    );

    let mesh = Mesh::new_from_wasm(mesh_indices, mesh_vertices, mesh_normals);
    let wireframe =
        wireframe_vertices.map(|vertices| Wireframe::new_from_wasm(wireframe_indices, vertices));

    let scene = scene::Scene::new_from_wasm(
        canvas_width,
        canvas_height,
        view_matrix,
        projection_matrix,
        mesh_world_matrix,
    );

    // log!("Scene: {}", scene);

    let segments = find_categorized_line_segments(&mesh, &wireframe, &scene);

    screen_space_lines_to_fitted_svg(&segments, &svg_config)
}

pub fn find_categorized_line_segments(
    mesh: &Mesh,
    maybe_wireframe: &Option<Wireframe>,
    scene: &Scene,
) -> Vec<LineSegmentCategorized> {
    let start_edges = Instant::now();

    let mut edges = mesh.find_edge_lines(&scene, false);

    let duration_edges = start_edges.elapsed();

    if let Some(wireframe) = maybe_wireframe {
        edges.append(&mut wireframe.edges());
    }

    let start_projection = Instant::now();
    let projected = scene.project_lines(&edges);
    let duration_projection = start_projection.elapsed();
    eprintln!("projected lines size: {}", projected.len());
    let start_deduplication = Instant::now();
    let deduped = dedupe_lines_faster(projected);
    let duration_deduplication = start_deduplication.elapsed();
    eprintln!("deduped lines size: {}", deduped.len());

    let start_splitting = Instant::now();
    let split_lines = split_lines_by_intersection(&deduped);
    let duration_splitting = start_splitting.elapsed();

    let start_checking_visibility = Instant::now();
    let segments = partition_visibility(mesh, scene, &split_lines);

    let duration_checking_visibility = start_checking_visibility.elapsed();

    let total = start_edges.elapsed();

    eprintln!(
        "find_edge_lines took {:?}, {:?}%",
        duration_edges,
        duration_edges.as_nanos() as f32 / total.as_nanos() as f32 * 100.0
    );
    eprintln!(
        "project_lines took {:?}, {:?}%",
        duration_projection,
        duration_projection.as_nanos() as f32 / total.as_nanos() as f32 * 100.0
    );
    eprintln!(
        "dedupe_lines took {:?}, {:?}%",
        duration_deduplication,
        duration_deduplication.as_nanos() as f32 / total.as_nanos() as f32 * 100.0
    );
    eprintln!(
        "split_lines_by_intersection took {:?}, {:?}%",
        duration_splitting,
        duration_splitting.as_nanos() as f32 / total.as_nanos() as f32 * 100.0
    );
    eprintln!(
        "get_visibility took {:?}, {:?}%",
        duration_checking_visibility,
        duration_checking_visibility.as_nanos() as f32 / total.as_nanos() as f32 * 100.0
    );
    eprintln!("overall took {:?}", total);

    segments
}

pub fn partition_visibility(
    mesh: &Mesh,
    scene: &Scene,
    split_lines: &Vec<ProjectedSplitLine>,
) -> Vec<LineSegmentCategorized> {
    let mut ray = Ray::new();
    let segments: Vec<LineSegmentCategorized> = split_lines
        .into_iter()
        .flat_map(|projected_line| {
            let culled: Vec<LineSegmentCategorized> = projected_line
                .split_screen_space_lines
                .iter()
                .enumerate()
                .map(|(_j, line_segment)| LineSegmentCategorized {
                    visibility: get_visibility(
                        &line_segment,
                        &projected_line.projected_line,
                        &scene,
                        &mut ray,
                        &mesh,
                    ),
                    line_segment: line_segment.to_owned(),
                })
                .collect();

            culled
        })
        .collect();
    segments
}
