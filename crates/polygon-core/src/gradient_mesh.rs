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
    let mut combined = ColoredMesh::default();
    let arc = if matches!(mode, GradientSampleMode::AlongPath) {
        Some(arc_length_table(path)?)
    } else {
        None
    };
    for outline in &outlines {
        let mesh = tessellate_path(outline, 0.25, TessFillRule::NonZero)?;
        let colored = match (&mode, &arc) {
            (GradientSampleMode::AlongPath, Some(table)) => colorize_along(mesh, gradient, table)?,
            _ => colorize_spatial(mesh, gradient)?,
        };
        append_colored(&mut combined, colored)?;
    }
    Ok(combined)
}

fn colorize_spatial(mesh: FillMesh, gradient: &Gradient) -> Result<ColoredMesh> {
    let colors = mesh
        .positions
        .iter()
        .map(|p| gradient.sample_at(*p).to_rgba())
        .collect();
    Ok(ColoredMesh {
        positions: mesh.positions,
        colors,
        indices: mesh.indices,
    })
}

fn colorize_along(
    mesh: FillMesh,
    gradient: &Gradient,
    table: &[(f64, [f64; 2])],
) -> Result<ColoredMesh> {
    check(!table.is_empty(), "Empty arc-length table")?;
    let total = table.last().map(|e| e.0).unwrap_or(0.0).max(1e-12);
    let colors = mesh
        .positions
        .iter()
        .map(|p| {
            let t = nearest_arc_t(*p, table) / total;
            gradient.sample(t).to_rgba()
        })
        .collect();
    Ok(ColoredMesh {
        positions: mesh.positions,
        colors,
        indices: mesh.indices,
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

fn append_colored(dst: &mut ColoredMesh, src: ColoredMesh) -> Result<()> {
    let base = dst.positions.len() as u32;
    check(
        dst.colors.len() == dst.positions.len(),
        "ColoredMesh color/position mismatch",
    )?;
    check(
        src.colors.len() == src.positions.len(),
        "ColoredMesh color/position mismatch",
    )?;
    dst.positions.extend(src.positions);
    dst.colors.extend(src.colors);
    dst.indices
        .extend(src.indices.into_iter().map(|i| i + base));
    Ok(())
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
    use crate::appearance::{Gradient, GradientStop, Color};

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
}
