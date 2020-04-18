use crate::lines::{LineSegment2, LineSegmentCategorized, LineVisibility};
use na::{Point2, Vector2};

pub struct SvgLineConfig {
    pub stroke_width: i32,
    pub stroke: String,
}

pub struct SvgConfig {
    pub width: i32,
    pub height: i32,
    pub margin: i32,
    pub visible: SvgLineConfig,
    pub obscured: Option<SvgLineConfig>,
    pub fit_lines: bool,
    pub source_canvas_width: i32,
    pub source_canvas_height: i32,
}

impl SvgConfig {
    pub fn new_default(source_canvas_width: i32, source_canvas_height: i32) -> SvgConfig {
        SvgConfig::new(
            source_canvas_width,
            source_canvas_height,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
        )
    }

    pub fn new(
        source_canvas_width: i32,
        source_canvas_height: i32,
        svg_config_width: Option<i32>,
        svg_config_height: Option<i32>,
        svg_config_margin: Option<i32>,
        svg_config_visible_stroke_width: Option<i32>,
        svg_config_visible_stroke: Option<String>,
        svg_config_hide_obscured: Option<bool>,
        svg_config_obscured_stroke_width: Option<i32>,
        svg_config_obscured_stroke: Option<String>,
        svg_config_fit_lines: Option<bool>,
    ) -> SvgConfig {
        let fit_lines = svg_config_fit_lines.unwrap_or(true);

        SvgConfig {
            source_canvas_width,
            source_canvas_height,
            width: svg_config_width.unwrap_or(source_canvas_width),
            height: svg_config_height.unwrap_or(source_canvas_height),
            margin: if fit_lines {
                svg_config_margin.unwrap_or(100)
            } else {
                0
            },
            visible: SvgLineConfig {
                stroke_width: svg_config_visible_stroke_width.unwrap_or(4),
                stroke: svg_config_visible_stroke.unwrap_or("black".to_owned()),
            },
            obscured: match svg_config_hide_obscured {
                Some(false) | None => Some(SvgLineConfig {
                    stroke_width: svg_config_obscured_stroke_width.unwrap_or(2),
                    stroke: svg_config_obscured_stroke.unwrap_or("grey".to_owned()),
                }),
                Some(true) => None,
            },
            fit_lines,
        }
    }
}

pub fn scale_screen_space_lines(
    screen_space_lines: &[LineSegmentCategorized],
    svg_config: &SvgConfig,
) -> Vec<LineSegmentCategorized> {
    let (min_bound, max_bound) = if svg_config.fit_lines {
        let all_points: Vec<Point2<f32>> = screen_space_lines
            .iter()
            .flat_map(|seg| vec![seg.line_segment.from, seg.line_segment.to])
            .collect();

        let all_x_values: Vec<f32> = all_points.iter().map(|p| p.x).collect();
        let max_x = all_x_values.iter().cloned().fold(std::f32::NAN, f32::max);
        let min_x = all_x_values.iter().cloned().fold(std::f32::NAN, f32::min);
        let all_y_values: Vec<f32> = all_points.iter().map(|p| p.y).collect();
        let max_y = all_y_values.iter().cloned().fold(std::f32::NAN, f32::max);
        let min_y = all_y_values.iter().cloned().fold(std::f32::NAN, f32::min);

        (Vector2::new(min_x, min_y), Vector2::new(max_x, max_y))
    } else {
        (
            Vector2::new(0.0, 0.0),
            Vector2::new(
                svg_config.source_canvas_width as f32,
                svg_config.source_canvas_height as f32,
            ),
        )
    };

    let margin = Vector2::new(svg_config.margin as f32, svg_config.margin as f32);

    let canvas = Vector2::new(svg_config.width as f32, svg_config.height as f32) - margin;

    let viewport = max_bound - min_bound;
    let half_viewport = (viewport * 0.5) + min_bound;
    let half_canvas = (canvas * 0.5) + (margin * 0.5);

    let x_scale = canvas.x / viewport.x;
    let y_scale = canvas.y / viewport.y;
    let scale = f32::min(x_scale, y_scale);

    let scaled_points = screen_space_lines
        .iter()
        .map(|line| LineSegmentCategorized {
            visibility: line.visibility,
            line_segment: LineSegment2 {
                from: ((&line.line_segment.from - half_viewport) * scale) + half_canvas,
                to: ((&line.line_segment.to - half_viewport) * scale) + half_canvas,
            },
        })
        .collect();

    scaled_points
}

pub fn screen_space_lines_to_fitted_svg(
    screen_space_lines: &[LineSegmentCategorized],
    svg_config: &SvgConfig,
) -> String {
    let scaled = scale_screen_space_lines(screen_space_lines, svg_config);
    line_segments_to_svg(&scaled, svg_config)
}

fn line_segments_to_svg(segments: &[LineSegmentCategorized], config: &SvgConfig) -> String {
    let (visible, obscured) = segments.iter().partition(|&seg| match seg.visibility {
        LineVisibility::VISIBLE => true,
        _ => false,
    });

    format!(
        "<svg viewBox=\"0 0 {width} {height}\" xmlns=\"http://www.w3.org/2000/svg\">
{obscured}
{visible}
</svg>",
        width = &config.width,
        height = &config.height,
        visible = create_path_element(visible, &config.visible),
        obscured = match &config.obscured {
            Some(conf) => create_path_element(obscured, conf),
            None => "".to_owned(),
        }
    )
}

fn create_path_element(lines: Vec<LineSegmentCategorized>, line_config: &SvgLineConfig) -> String {
    let mut path_def = "".to_string();
    let mut current: Option<Point2<f32>> = None;

    for (_i, line) in lines.iter().enumerate() {
        let start = &line.line_segment.from;
        let end = &line.line_segment.to;

        match current {
            Some(current) if relative_eq!(current, start) => {
                path_def.push_str("L ");
            }
            _ => {
                path_def.push_str(format!("M {x} {y} ", x = start.x, y = start.y).as_str());
            }
        }

        path_def.push_str(format!("{x} {y}", x = end.x, y = end.y).as_str());

        current = Some(end.to_owned());
    }

    format!("<path d=\"{path_def}\" stroke=\"{stroke}\" fill=\"none\" stroke-width=\"{stroke_width}\" stroke-linecap=\"round\" stroke-linejoin=\"round\" />", path_def=path_def, stroke=line_config.stroke, stroke_width=line_config.stroke_width)
}
