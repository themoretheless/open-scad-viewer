//! Gradient-colored fill/stroke meshes for planar paths (no renderer).
use crate::appearance::Gradient;
use crate::{Result, check};
use planar_geometry::path::BezierPath;
use planar_geometry::stroke::{StrokeOptions, outline_stroke};
use planar_geometry::tessellation::{
    FillMesh, FillRule as TessFillRule, tessellate_path, tessellate_rings,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum GradientSampleMode {
    /// Sample gradient in document XY via [`Gradient::sample_at`].
    #[default]
    Spatial,
    /// Sample by normalized arc-length along the source path (stroke ribbons).
    AlongPath,
}

/// Triangle mesh with per-vertex sRGB+A colors.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct ColoredMesh {
    pub positions: Vec<[f64; 2]>,
    pub colors: Vec<[u8; 4]>,
    pub indices: Vec<u32>,
}

impl ColoredMesh {
    pub fn triangle_count(&self) -> usize {
        self.indices.len() / 3
    }
}

/// Tessellate a closed path and color vertices from `gradient` (spatial).
pub fn gradient_fill_mesh(
    path: &BezierPath,
    gradient: &Gradient,
    tolerance: f64,
    even_odd: bool,
) -> Result<ColoredMesh> {
    let rule = if even_odd {
        TessFillRule::EvenOdd
    } else {
        TessFillRule::NonZero
    };
    let mesh = tessellate_path(path, tolerance, rule)?;
    colorize_spatial(mesh, gradient)
}

/// Tessellate closed rings and color vertices spatially.
pub fn gradient_fill_rings(
    rings: &planar_geometry::rings::Rings,
    gradient: &Gradient,
    even_odd: bool,
) -> Result<ColoredMesh> {
    let rule = if even_odd {
        TessFillRule::EvenOdd
    } else {
        TessFillRule::NonZero
    };
    let mesh = tessellate_rings(rings, rule)?;
    colorize_spatial(mesh, gradient)
}

/// Expand stroke to outline paths, tessellate, color by spatial or along-path mode.
pub fn gradient_stroke_mesh(
    path: &BezierPath,
    opts: &StrokeOptions,
    gradient: &Gradient,
    mode: GradientSampleMode,
) -> Result<ColoredMesh> {
    let outlines = outline_stroke(path, opts)?;
    check(!outlines.is_empty(), "Gradient stroke produced no outline")?;
    let rings = outlines
        .iter()
        .map(|p| p.to_ring(0.25))
        .collect::<Result<Vec<_>>>()?;
    let mesh = tessellate_rings(&rings, TessFillRule::NonZero)?;
    match mode {
        GradientSampleMode::AlongPath => colorize_along(mesh, gradient, &arc_length_table(path)?),
        GradientSampleMode::Spatial => colorize_spatial(mesh, gradient),
    }
}

fn colorize_spatial(mesh: FillMesh, gradient: &Gradient) -> Result<ColoredMesh> {
    use crate::appearance::{GradientKind, GradientSpread};
    let max_edge = match gradient.kind {
        GradientKind::Linear { p1, p2 }
            if gradient.spread != GradientSpread::Pad || gradient.stops.len() > 2 =>
        {
            (p2[0] - p1[0]).hypot(p2[1] - p1[1]) * 0.125
        }
        GradientKind::Radial { radius, .. } => radius.abs() * 0.25,
        _ => f64::INFINITY,
    };
    let seams = gradient_seams(&mesh.positions, gradient);
    let max_edge = if matches!(gradient.kind, GradientKind::Linear { .. })
        && !seams.is_empty()
        && seams.len() < 1024
    {
        f64::INFINITY
    } else {
        max_edge
    };
    sample_colors(mesh, max_edge, &seams, |p| gradient.sample_at(p).to_rgba())
}

fn gradient_seams(positions: &[[f64; 2]], gradient: &Gradient) -> Vec<[f64; 3]> {
    use crate::appearance::{GradientKind, GradientSpread};
    match gradient.kind {
        GradientKind::Linear { p1, p2 } => {
            let (dx, dy) = (p2[0] - p1[0], p2[1] - p1[1]);
            let d = dx * dx + dy * dy;
            if d < 1e-18 {
                return Vec::new();
            }
            let (a, b, c) = (dx / d, dy / d, -(dx * p1[0] + dy * p1[1]) / d);
            let (min, max) =
                positions
                    .iter()
                    .fold((f64::INFINITY, f64::NEG_INFINITY), |(lo, hi), p| {
                        let t = a * p[0] + b * p[1] + c;
                        (lo.min(t), hi.max(t))
                    });
            let mut values = Vec::new();
            match gradient.spread {
                GradientSpread::Pad => values.extend(gradient.stops.iter().map(|s| s.offset)),
                _ if max - min <= 128. => {
                    for k in min.floor() as i32 - 1..=max.ceil() as i32 + 1 {
                        values.push(k as f64);
                        for s in &gradient.stops {
                            values.push(
                                k as f64
                                    + if gradient.spread == GradientSpread::Reflect
                                        && k.rem_euclid(2) == 1
                                    {
                                        1. - s.offset
                                    } else {
                                        s.offset
                                    },
                            );
                        }
                    }
                }
                _ => {}
            }
            values.sort_by(f64::total_cmp);
            values.dedup_by(|x, y| (*x - *y).abs() < 1e-12);
            values
                .into_iter()
                .filter(|t| *t >= min - 1e-9 && *t <= max + 1e-9)
                .take(1024)
                .map(|t| [a, b, c - t])
                .collect()
        }
        GradientKind::Conic { center, angle } => {
            let (a, b) = (-angle.sin(), angle.cos());
            vec![[a, b, -a * center[0] - b * center[1]]]
        }
        _ => Vec::new(),
    }
}

fn sample_colors(
    mesh: FillMesh,
    max_edge: f64,
    seams: &[[f64; 3]],
    sample: impl Fn([f64; 2]) -> [u8; 4],
) -> Result<ColoredMesh> {
    use planar_geometry::attribute_mesh::{SampleOptions, sample_mesh_with_seams};
    let bounds = mesh.positions.iter().fold(
        ([f64::INFINITY; 2], [f64::NEG_INFINITY; 2]),
        |(mut lo, mut hi), p| {
            for i in 0..2 {
                lo[i] = lo[i].min(p[i]);
                hi[i] = hi[i].max(p[i]);
            }
            (lo, hi)
        },
    );
    let min_edge = ((bounds.1[0] - bounds.0[0]).max(bounds.1[1] - bounds.0[1]) / 4096.).max(1e-9);
    let result = sample_mesh_with_seams(
        &mesh,
        &SampleOptions {
            max_edge_length: max_edge.max(min_edge),
            min_edge_length: min_edge,
            ..Default::default()
        },
        seams,
        |p| sample(p).map(f64::from),
    )?;
    Ok(ColoredMesh {
        positions: result.positions,
        colors: result
            .values
            .into_iter()
            .map(|v| v.map(|x| x.round().clamp(0., 255.) as u8))
            .collect(),
        indices: result.indices,
    })
}

fn colorize_along(
    mesh: FillMesh,
    gradient: &Gradient,
    table: &[(f64, [f64; 2])],
) -> Result<ColoredMesh> {
    check(!table.is_empty(), "Empty arc-length table")?;
    let total = table.last().map(|e| e.0).unwrap_or(0.0).max(1e-12);
    let max_edge = if gradient.spread == crate::appearance::GradientSpread::Pad {
        f64::INFINITY
    } else {
        total / 32.
    };
    sample_colors(mesh, max_edge, &[], |p| {
        gradient.sample(nearest_arc_t(p, table) / total).to_rgba()
    })
}

fn arc_length_table(path: &BezierPath) -> Result<Vec<(f64, [f64; 2])>> {
    let pts = path.flatten()?;
    check(pts.len() >= 2, "Path too short for arc table")?;
    let mut out = Vec::with_capacity(pts.len());
    let mut acc = 0.0;
    out.push((0.0, pts[0]));
    for w in pts.windows(2) {
        let d = ((w[1][0] - w[0][0]).powi(2) + (w[1][1] - w[0][1]).powi(2)).sqrt();
        acc += d;
        out.push((acc, w[1]));
    }
    Ok(out)
}

fn nearest_arc_t(p: [f64; 2], table: &[(f64, [f64; 2])]) -> f64 {
    let mut best_t = 0.0;
    let mut best_d = f64::INFINITY;
    for window in table.windows(2) {
        let (t0, a) = window[0];
        let (t1, b) = window[1];
        let ab = [b[0] - a[0], b[1] - a[1]];
        let ap = [p[0] - a[0], p[1] - a[1]];
        let l2 = ab[0] * ab[0] + ab[1] * ab[1];
        let u = if l2 < 1e-18 {
            0.0
        } else {
            ((ap[0] * ab[0] + ap[1] * ab[1]) / l2).clamp(0.0, 1.0)
        };
        let q = [a[0] + ab[0] * u, a[1] + ab[1] * u];
        let d = (p[0] - q[0]).hypot(p[1] - q[1]);
        if d < best_d {
            best_d = d;
            best_t = t0 + (t1 - t0) * u;
        }
    }
    best_t
}

/// Premultiply vertex colors by document opacity in `[0,1]`.
pub fn apply_opacity(mesh: &mut ColoredMesh, opacity: f64) {
    let o = opacity.clamp(0.0, 1.0);
    for c in &mut mesh.colors {
        let a = (f64::from(c[3]) * o).round().clamp(0.0, 255.0) as u8;
        let scale = if c[3] == 0 {
            0.0
        } else {
            f64::from(a) / f64::from(c[3])
        };
        c[0] = (f64::from(c[0]) * scale).round().clamp(0.0, 255.0) as u8;
        c[1] = (f64::from(c[1]) * scale).round().clamp(0.0, 255.0) as u8;
        c[2] = (f64::from(c[2]) * scale).round().clamp(0.0, 255.0) as u8;
        c[3] = a;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::appearance::{Color, Gradient, GradientStop};

    #[test]
    fn linear_fill_mesh_has_vertex_colors() {
        let path = BezierPath::from_rect([0., 0.], [10., 4.]).unwrap();
        let g = Gradient::linear(
            [0., 0.],
            [10., 0.],
            vec![
                GradientStop {
                    offset: 0.0,
                    color: Color::rgb(255, 0, 0),
                },
                GradientStop {
                    offset: 1.0,
                    color: Color::rgb(0, 0, 255),
                },
            ],
        );
        let mesh = gradient_fill_mesh(&path, &g, 0.25, false).unwrap();
        assert_eq!(mesh.triangle_count(), 2);
        assert_eq!(mesh.colors.len(), mesh.positions.len());
        // Left side red-ish, right side blue-ish.
        let left = mesh
            .positions
            .iter()
            .zip(mesh.colors.iter())
            .filter(|(p, _)| p[0] < 1.0)
            .map(|(_, c)| c[0])
            .max()
            .unwrap();
        let right = mesh
            .positions
            .iter()
            .zip(mesh.colors.iter())
            .filter(|(p, _)| p[0] > 9.0)
            .map(|(_, c)| c[2])
            .max()
            .unwrap();
        assert!(left > 200);
        assert!(right > 200);
    }

    #[test]
    fn stroke_along_path_colors_endpoints() {
        let path = BezierPath::from_polyline(&[[0., 0.], [20., 0.]], false).unwrap();
        let g = Gradient::linear(
            [0., 0.],
            [1., 0.],
            vec![
                GradientStop {
                    offset: 0.0,
                    color: Color::rgb(0, 255, 0),
                },
                GradientStop {
                    offset: 1.0,
                    color: Color::rgb(255, 0, 0),
                },
            ],
        );
        let mesh = gradient_stroke_mesh(
            &path,
            &StrokeOptions {
                width: 2.0,
                ..Default::default()
            },
            &g,
            GradientSampleMode::AlongPath,
        )
        .unwrap();
        assert!(mesh.triangle_count() >= 2);
        assert_eq!(mesh.colors.len(), mesh.positions.len());
    }

    #[test]
    fn radial_fill_samples_interior_instead_of_only_saturated_corners() {
        use crate::appearance::{GradientKind, GradientSpread};
        let path = BezierPath::from_rect([0., 0.], [1., 1.]).unwrap();
        let gradient = Gradient {
            kind: GradientKind::Radial {
                center: [0.5, 0.5],
                radius: 0.5,
            },
            spread: GradientSpread::Pad,
            stops: vec![
                GradientStop {
                    offset: 0.,
                    color: Color::rgb(255, 255, 255),
                },
                GradientStop {
                    offset: 1.,
                    color: Color::rgb(0, 0, 0),
                },
            ],
        };
        let mesh = gradient_fill_mesh(&path, &gradient, 0.01, false).unwrap();
        assert!(mesh.triangle_count() > 2);
        assert!(
            mesh.positions
                .iter()
                .zip(&mesh.colors)
                .any(|(p, c)| (p[0] - 0.5).hypot(p[1] - 0.5) < 1e-8 && c[0] == 255)
        );
    }
}
