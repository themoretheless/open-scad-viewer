//! Typed planar sketch transforms with retained analytic circles and arcs.
//!
//! An explicit pivot takes precedence over an analytic center and the centroid.
//! Analytic curves retain their sweep and are sampled again after transforming.
//! ```
//! use planar_geometry::sketch_transform::{transform, Transform};
//! let result = transform(&[[0., 0.], [2., 0.]], None, Transform {
//!     delta: [3., 4.], angle: 0., scale: 2., pivot: None,
//! }).unwrap();
//! assert_eq!(result.points, vec![[2., 4.], [6., 4.]]);
//! ```
use crate::sketch_shapes::{self, CurveKind};
use math_core::{Error, Result};
fn fail(message: impl Into<String>) -> Error {
    Error::new("GEOMETRY_INVALID_INPUT", message)
}
fn finite(values: impl IntoIterator<Item = f64>) -> Result<()> {
    if values.into_iter().all(f64::is_finite) {
        Ok(())
    } else {
        Err(fail("Sketch geometry exceeds finite numeric range."))
    }
}
#[derive(Clone, Copy, Debug)]
pub struct AnalyticCurve {
    pub kind: CurveKind,
    pub center: [f64; 2],
    pub radius: f64,
    pub start: f64,
    pub sweep: f64,
}
impl AnalyticCurve {
    pub fn sample(&self) -> Result<Vec<[f64; 2]>> {
        sketch_shapes::sample(self.kind, self.center, self.radius, self.start, self.sweep)
    }
}
#[derive(Clone, Copy, Debug)]
pub struct Transform {
    pub delta: [f64; 2],
    pub angle: f64,
    pub scale: f64,
    pub pivot: Option<[f64; 2]>,
}
#[derive(Clone, Debug)]
pub struct Geometry {
    pub points: Vec<[f64; 2]>,
    pub analytic: Option<AnalyticCurve>,
}
pub fn transform(
    points: &[[f64; 2]],
    analytic: Option<AnalyticCurve>,
    options: Transform,
) -> Result<Geometry> {
    let Transform {
        delta,
        angle,
        scale,
        pivot,
    } = options;
    if !delta.into_iter().chain([angle, scale]).all(f64::is_finite) || scale <= 0. {
        return Err(fail("Invalid sketch transform."));
    }
    if points.is_empty() {
        return Err(fail("Sketch requires points."));
    }
    for &p in points {
        finite(p)?;
    }
    let center = pivot
        .or_else(|| analytic.map(|a| a.center))
        .unwrap_or_else(|| {
            std::array::from_fn(|k| points.iter().map(|p| p[k] / points.len() as f64).sum())
        });
    finite(center)?;
    let (s, c) = angle.to_radians().sin_cos();
    let apply = |p: [f64; 2]| -> [f64; 2] {
        let x = p[0] - center[0];
        let y = p[1] - center[1];
        [
            center[0] + delta[0] + scale * (x * c - y * s),
            center[1] + delta[1] + scale * (x * s + y * c),
        ]
    };
    let mut moved = points.iter().copied().map(apply).collect::<Vec<_>>();
    finite(moved.iter().flatten().copied())?;
    let analytic = if let Some(curve) = analytic {
        let center = apply(curve.center);
        let radius = curve.radius * scale;
        let start = curve.start + angle;
        finite(center.into_iter().chain([radius, start]))?;
        let curve = AnalyticCurve {
            center,
            radius,
            start,
            ..curve
        };
        moved = curve.sample()?;
        Some(curve)
    } else {
        None
    };
    Ok(Geometry {
        points: moved,
        analytic,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn analytic_transform_uses_explicit_pivot_and_retains_sweep() {
        let curve = AnalyticCurve {
            kind: CurveKind::Arc,
            center: [2., 0.],
            radius: 1.,
            start: 0.,
            sweep: 90.,
        };
        let p = curve.sample().unwrap();
        let result = transform(
            &p,
            Some(curve),
            Transform {
                delta: [1., 3.],
                angle: 90.,
                scale: 2.,
                pivot: Some([0., 0.]),
            },
        )
        .unwrap();
        let next = result.analytic.unwrap();
        assert!((next.center[0] - 1.).abs() < 1e-12);
        assert_eq!(next.center[1], 7.);
        assert_eq!(next.radius, 2.);
        assert_eq!(next.start, 90.);
        assert_eq!(next.sweep, 90.);
        assert_eq!(result.points, next.sample().unwrap());
    }
    #[test]
    fn centroid_rotation_and_invalid_scale() {
        let p = [[0., 0.], [2., 0.]];
        let q = transform(
            &p,
            None,
            Transform {
                delta: [0., 0.],
                angle: 180.,
                scale: 1.,
                pivot: None,
            },
        )
        .unwrap();
        assert!((q.points[0][0] - 2.).abs() < 1e-12);
        assert!(q.points[1][0].abs() < 1e-12);
        assert!(
            transform(
                &p,
                None,
                Transform {
                    delta: [0., 0.],
                    angle: 0.,
                    scale: 0.,
                    pivot: None
                }
            )
            .is_err()
        );
    }
}
