//! Wall thickness estimated by inward rays from triangle centroids.

use super::{DiagnosticsError, Mesh, P, triangle_tree};

/// Parameters of [`wall_thickness`].
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ThicknessOptions {
    /// Samples thinner than this are flagged (model units, `>= 0`).
    pub min_thickness: f64,
    /// Maximum number of rays (one per non-degenerate triangle); more fails
    /// with [`DiagnosticsError::WorkBudgetExceeded`].
    pub max_rays: usize,
}

impl Default for ThicknessOptions {
    fn default() -> Self {
        Self {
            min_thickness: 0.0,
            max_rays: 1_000_000,
        }
    }
}

/// Sampled wall thickness.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct ThicknessReport {
    /// Smallest sampled thickness, `None` when no ray hit the far wall.
    pub min_thickness: Option<f64>,
    /// Largest sampled thickness.
    pub max_thickness: Option<f64>,
    /// Triangle whose centroid ray gave `min_thickness`.
    pub min_triangle: Option<usize>,
    /// Rays cast.
    pub samples: usize,
    /// Rays that left the solid without a hit (open or inverted surface).
    pub missed: usize,
    /// Triangles whose sample is below `min_thickness`, ascending.
    pub thin_triangles: Vec<usize>,
}

impl ThicknessReport {
    /// True when any sample is thinner than the requested threshold.
    pub fn below_threshold(&self) -> bool {
        !self.thin_triangles.is_empty()
    }
}

fn sub(a: P, b: P) -> P {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}

fn cross(a: P, b: P) -> P {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}

fn dot(a: P, b: P) -> f64 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}

/// Ray parameter of the hit with triangle `t` (Moller-Trumbore), if any.
fn ray_triangle(o: P, d: P, t: &[P; 3]) -> Option<f64> {
    let e1 = sub(t[1], t[0]);
    let e2 = sub(t[2], t[0]);
    let h = cross(d, e2);
    let det = dot(e1, h);
    if det.abs() < 1e-300 {
        return None;
    }
    let s = sub(o, t[0]);
    let u = dot(s, h) / det;
    let q = cross(s, e1);
    let v = dot(d, q) / det;
    let tol = 1e-12;
    if u < -tol || v < -tol || u + v > 1.0 + tol {
        return None;
    }
    Some(dot(e2, q) / det)
}

fn ray_box(o: P, inv: P, min: P, max: P, best: f64) -> bool {
    let (mut lo, mut hi) = (0.0f64, best);
    for k in 0..3 {
        let a = (min[k] - o[k]) * inv[k];
        let b = (max[k] - o[k]) * inv[k];
        let (a, b) = if a <= b { (a, b) } else { (b, a) };
        // NaN from 0 * inf (origin on a slab plane, axis-parallel ray) keeps
        // the box: `max`/`min` ignore NaN operands.
        lo = lo.max(a);
        hi = hi.min(b);
    }
    lo <= hi
}

/// Estimate wall thickness: from each triangle centroid cast a ray against
/// the outward normal and take the distance to the nearest other triangle.
///
/// The outward side follows the sign of the enclosed volume, so inward
/// oriented solids work too. This is a sampled estimate, not a medial-axis
/// proof: walls thinner than the sampling between centroids can be missed.
pub fn wall_thickness(
    positions: &[f64],
    indices: &[usize],
    options: ThicknessOptions,
) -> Result<ThicknessReport, DiagnosticsError> {
    if !options.min_thickness.is_finite() || options.min_thickness < 0.0 {
        return Err(DiagnosticsError::InvalidParameter("min_thickness"));
    }
    let mesh = Mesh::new(positions, indices)?;
    let Some(tree) = triangle_tree(&mesh, 0.0, |_| true) else {
        return Ok(ThicknessReport::default());
    };
    let samples: Vec<usize> = (0..mesh.triangles.len())
        .filter(|&t| !mesh.is_degenerate(t))
        .collect();
    if samples.len() > options.max_rays {
        return Err(DiagnosticsError::WorkBudgetExceeded {
            stage: "wall thickness rays",
            budget: options.max_rays,
            required: samples.len(),
        });
    }
    let volume: f64 = samples
        .iter()
        .map(|&t| {
            let [a, b, c] = mesh.corners(t);
            dot(a, cross(b, c))
        })
        .sum();
    let inward = if volume < 0.0 { 1.0 } else { -1.0 };
    let mut report = ThicknessReport {
        samples: samples.len(),
        ..Default::default()
    };
    for &t in &samples {
        let c = mesh.corners(t);
        let n = cross(sub(c[1], c[0]), sub(c[2], c[0]));
        let len = dot(n, n).sqrt();
        let d = n.map(|v| inward * v / len);
        let o: P = std::array::from_fn(|k| (c[0][k] + c[1][k] + c[2][k]) / 3.0);
        let inv = d.map(|v| 1.0 / v);
        let scale = c
            .iter()
            .flatten()
            .fold(0.0f64, |m, v| m.max(v.abs()))
            .max(1.0);
        let eps = scale * 1e-12;
        let nearest = std::cell::Cell::new(f64::INFINITY);
        tree.walk(
            |min, max| ray_box(o, inv, min, max, nearest.get()),
            |item| {
                if item.index == t {
                    return;
                }
                if let Some(s) = ray_triangle(o, d, &mesh.corners(item.index))
                    && s > eps
                    && s < nearest.get()
                {
                    nearest.set(s);
                }
            },
        );
        let best = nearest.get();
        if !best.is_finite() {
            report.missed += 1;
            continue;
        }
        if report.min_thickness.is_none_or(|m| best < m) {
            report.min_thickness = Some(best);
            report.min_triangle = Some(t);
        }
        report.max_thickness = Some(report.max_thickness.map_or(best, |m| m.max(best)));
        if best < options.min_thickness {
            report.thin_triangles.push(t);
        }
    }
    Ok(report)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::diagnostics::tests::{box_mesh, flip, merge};

    #[test]
    fn slab_thickness_is_its_smallest_extent() {
        let (p, i) = box_mesh([0.; 3], [10., 10., 0.5]);
        let r = wall_thickness(
            &p,
            &i,
            ThicknessOptions {
                min_thickness: 1.0,
                ..Default::default()
            },
        )
        .unwrap();
        assert!((r.min_thickness.unwrap() - 0.5).abs() < 1e-9, "{r:?}");
        assert!((r.max_thickness.unwrap() - 10.0).abs() < 1e-9);
        assert_eq!(r.missed, 0);
        assert!(r.below_threshold());
        // Only the top and bottom faces (4 triangles) see the 0.5 wall.
        assert_eq!(r.thin_triangles.len(), 4);
    }

    #[test]
    fn inverted_orientation_measures_the_same() {
        let (p, i) = box_mesh([0.; 3], [2., 3., 4.]);
        let a = wall_thickness(&p, &i, Default::default()).unwrap();
        let b = wall_thickness(&p, &flip(&i), Default::default()).unwrap();
        assert_eq!(a.min_thickness, b.min_thickness);
        assert!((a.min_thickness.unwrap() - 2.0).abs() < 1e-9);
        assert!(!a.below_threshold());
    }

    #[test]
    fn hollow_box_reports_the_shell_wall() {
        // Outer box with an inverted inner box: a closed shell 0.25 thick.
        let (p, i) = merge(box_mesh([0.; 3], [4.; 3]), box_mesh([0.25; 3], [3.75; 3]));
        let mut i = i;
        let inner = flip(&i[36..]);
        i.truncate(36);
        i.extend(inner);
        let r = wall_thickness(
            &p,
            &i,
            ThicknessOptions {
                min_thickness: 0.3,
                ..Default::default()
            },
        )
        .unwrap();
        assert!((r.min_thickness.unwrap() - 0.25).abs() < 1e-9, "{r:?}");
        assert_eq!(r.thin_triangles.len(), 24);
    }

    #[test]
    fn rejects_bad_threshold_and_budget() {
        let (p, i) = box_mesh([0.; 3], [1.; 3]);
        let bad = ThicknessOptions {
            min_thickness: f64::NAN,
            ..Default::default()
        };
        assert_eq!(
            wall_thickness(&p, &i, bad),
            Err(DiagnosticsError::InvalidParameter("min_thickness"))
        );
        let tight = ThicknessOptions {
            max_rays: 5,
            ..Default::default()
        };
        assert!(matches!(
            wall_thickness(&p, &i, tight),
            Err(DiagnosticsError::WorkBudgetExceeded { required: 12, .. })
        ));
    }
}
