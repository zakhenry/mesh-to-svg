use std::fmt;
use std::fmt::{Display, Formatter};

use na::{distance_squared, Matrix4, Point2, Point3, Vector3};

use crate::lines::{LineSegment2, LineSegment3, ProjectedLine};
use crate::mesh::{Facet, Mesh};

pub struct Scene {
    pub width: f32,
    pub height: f32,
    pub view_matrix: Matrix4<f32>,
    pub projection_matrix: Matrix4<f32>,
    pub mesh_world_matrix: Matrix4<f32>,
}

impl Scene {
    pub fn new_from_wasm(
        width: i32,
        height: i32,
        view: Box<[f32]>,
        projection: Box<[f32]>,
        mesh_world: Box<[f32]>,
    ) -> Scene {
        let view_matrix = Scene::matrix_from_boxed_float_array(view);
        let projection_matrix = Scene::matrix_from_boxed_float_array(projection);
        let mesh_world_matrix = Scene::matrix_from_boxed_float_array(mesh_world);

        Scene::new(
            width as f32,
            height as f32,
            view_matrix,
            projection_matrix,
            mesh_world_matrix,
        )
    }

    pub fn new(
        width: f32,
        height: f32,
        view_matrix: Matrix4<f32>,
        projection_matrix: Matrix4<f32>,
        mesh_world_matrix: Matrix4<f32>,
    ) -> Scene {
        Scene {
            width,
            height,
            view_matrix,
            projection_matrix,
            mesh_world_matrix,
        }
    }

    // @todo bench if this fixed value should be optimised by memoization or similar
    pub fn transformation_matrix(&self) -> Matrix4<f32> {
        &self.projection_matrix * &self.view_matrix * &self.mesh_world_matrix
    }

    // @todo bench if this fixed value should be optimised by memoization or similar
    // forward vector is the first three columns of the third row of the view matrix
    pub fn camera_forward_vector(&self) -> Vector3<f32> {
        Vector3::new(
            self.view_matrix[2].to_owned(),
            self.view_matrix[6].to_owned(),
            self.view_matrix[10].to_owned(),
        )
    }

    // these values are hardcoded by manually setting a value in babylonjs and reading out the values with `log!("{}", scene);`
    pub fn new_test() -> Scene {
        #[rustfmt::skip]
        let view_matrix = Matrix4::new(
            0.79758435,        0.0,   0.6032074,        0.0,
             0.2850845, 0.88126934, -0.37694982,        0.0,
            -0.5315882, 0.47261438,  0.70288664, -594.28314,
                   0.0,        0.0,         0.0,        1.0,
        );

        #[rustfmt::skip]
        let projection_matrix = Matrix4::new(
            0.021944271,         0.0,     0.0, 0.0,
                    0.0, 0.029259028,     0.0, 0.0,
                    0.0,         0.0, -0.0001, 0.0,
                    0.0,         0.0,     0.0, 1.0,
        );

        // Z-up
        #[rustfmt::skip]
        let mesh_world_matrix = Matrix4::new(
            1.0,    0.0,    0.0,    0.0,
            0.0,    0.0,    1.0,    0.0,
            0.0,   -1.0,    0.0,    0.0,
            0.0,    0.0,    0.0,    1.0,
        );

        // no transform
        // let mesh_world_matrix = Matrix4::identity();

        Scene::new(
            800.0,
            600.0,
            view_matrix,
            projection_matrix,
            mesh_world_matrix,
        )
    }

    fn matrix_from_boxed_float_array(data: Box<[f32]>) -> Matrix4<f32> {
        Matrix4::new(
            data[0], data[4], data[8], data[12], data[1], data[5], data[9], data[13], data[2],
            data[6], data[10], data[14], data[3], data[7], data[11], data[15],
        )
    }

    pub fn project_point(&self, point: &Point3<f32>) -> Point2<f32> {
        let transformed = &self.transformation_matrix().transform_point(&point);

        Point2::new(
            (transformed.x + 1.0) / 2.0 * self.width,
            (transformed.y - 1.0) / 2.0 * -self.height,
        )
    }

    pub fn unproject_point(&self, point: &Point2<f32>) -> Point3<f32> {
        let inverted = &self.transformation_matrix().try_inverse().unwrap();

        let projection_point = Point3::new(
            (point.x / self.width) * 2.0 - 1.0,
            -((point.y / self.height) * 2.0 - 1.0),
            -1.0,
        );

        inverted.transform_point(&projection_point)
    }

    pub fn project_line(&self, line: &LineSegment3) -> LineSegment2 {
        LineSegment2 {
            from: self.project_point(&line.from),
            to: self.project_point(&line.to),
        }
    }

    pub fn project_lines(&self, lines: &[LineSegment3]) -> Vec<ProjectedLine> {
        let projected_lines: Vec<ProjectedLine> = lines
            .into_iter()
            .map(|line| {
                let screen_space = self.project_line(line);

                ProjectedLine {
                    screen_space,
                    view_space: line.to_owned(),
                }
            })
            .collect();

        projected_lines
    }
}

impl Display for Scene {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Scene: \
width: {width}
height: {height}
view_matrix: {view_matrix}
projection_matrix: {projection_matrix}
mesh_world_matrix: {mesh_world_matrix}
camera_forward_vector: {camera_forward_vector}",
            width = self.width,
            height = self.height,
            view_matrix = self.view_matrix,
            projection_matrix = self.projection_matrix,
            mesh_world_matrix = self.mesh_world_matrix,
            camera_forward_vector = self.camera_forward_vector()
        )
    }
}

pub struct Ray {
    pub origin: Point3<f32>,
    pub direction: Vector3<f32>,
    pub length: f32,
}

impl Ray {
    pub fn new() -> Ray {
        Ray {
            origin: Point3::origin(),
            direction: Vector3::zeros(),
            length: 0.0,
        }
    }

    pub fn intersects_mesh(&self, mesh: &Mesh) -> bool {
        for facet in &mesh.facets {
            if let Some(distance) = self.intersects_facet(facet) {
                if distance > 0.01 {
                    return true;
                }
            }
        }

        false
    }

    fn intersects_facet(&self, facet: &Facet) -> Option<f32> {
        let length_squared = self.length * self.length;

        // if the ray does not reach the facet, it cannot intersect. Exit early
        if distance_squared(&self.origin, &facet.points[0]) > length_squared
            && distance_squared(&self.origin, &facet.points[1]) > length_squared
            && distance_squared(&self.origin, &facet.points[2]) > length_squared
        {
            return None;
        }

        let edge_1 = &facet.points[1] - &facet.points[0];
        let edge_2 = &facet.points[2] - &facet.points[0];

        let pvec: Vector3<f32> = self.direction.cross(&edge_2);

        let det = edge_1.dot(&pvec);

        let epsilon = 0.0000001;
        if det > -epsilon && det < epsilon {
            return None;
        }

        let invdet = 1.0 / det;

        let tvec = &self.origin - &facet.points[0];

        let bv = tvec.dot(&pvec) * invdet;

        if bv < 0.0 || bv > 1.0 {
            return None;
        }

        let qvec = tvec.cross(&edge_1);

        let bw = &self.direction.dot(&qvec) * invdet;

        if bw < 0.0 || bv + bw > 1.0 {
            return None;
        }

        let distance = edge_2.dot(&qvec) * invdet;

        if distance > self.length {
            return None;
        }

        Some(distance)
    }
}
