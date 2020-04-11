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

#[macro_use]
mod utils;
pub mod lines;
pub mod mesh;
pub mod scene;
pub mod svg_renderer;

// For the macro relative_eq!

#[wasm_bindgen]
pub fn mesh_to_svg_lines(
    canvas_width: i32,
    canvas_height: i32,
    mesh_indices: Box<[usize]>,
    mesh_vertices: Box<[f32]>,
    mesh_normals: Box<[f32]>,
    wireframe_indices: Box<[usize]>,
    wireframe_vertices: Box<[f32]>,
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
    let wireframe = Wireframe::new_from_wasm(wireframe_indices, wireframe_vertices);
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
    wireframe: &Wireframe,
    scene: &Scene,
) -> Vec<LineSegmentCategorized> {
    let mut edges = mesh.find_edge_lines(&scene, false);
    edges.append(&mut wireframe.edges());
    let projected = scene.project_lines(&edges);
    let split_lines = split_lines_by_intersection(projected);
    let mut ray = Ray::new(&mesh);
    let mut index: usize = 0;
    let segments: Vec<LineSegmentCategorized> = split_lines
        .iter()
        .flat_map(|projected_line| {
            let culled: Vec<LineSegmentCategorized> = projected_line
                .split_screen_space_lines
                .iter()
                .enumerate()
                .map(|(_j, line_segment)| {
                    let seg = LineSegmentCategorized {
                        visibility: get_visibility(
                            &line_segment,
                            &projected_line.projected_line,
                            &scene,
                            &mut ray,
                        ),
                        line_segment: line_segment.to_owned(),
                    };

                    index += 1;

                    seg
                })
                .collect();

            culled
        })
        .collect();
    segments
}
