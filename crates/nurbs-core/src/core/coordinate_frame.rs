//! Reversible coordinate frames and explicit unit scaling of control geometry.
use crate::{bounds::Bounds, check, curve::Curve, surface::Surface, Result};
#[derive(Clone, Debug, PartialEq)]
pub struct Frame {
    pub origin: Vec<f64>,
    /// Local coordinate = (world coordinate - origin) * scale.
    pub scale: f64,
}
impl Frame {
    /// Center a validated box and scale its longest extent to length two.
    /// A point box uses scale one. Extremely small extents may be refused.
    pub fn from_bounds(b: &Bounds) -> Result<Self> {
        check(
            (2..=3).contains(&b.min.len()) && b.max.len() == b.min.len(),
            "Frame requires a 2D or 3D box",
        )?;
        let mut origin = Vec::new();
        let mut extent = 0_f64;
        for (&a, &z) in b.min.iter().zip(&b.max) {
            check(
                a.is_finite() && z.is_finite() && a <= z && (z - a).is_finite(),
                "Frame bounds must be finite and ordered",
            )?;
            origin.push(a + (z - a) / 2.);
            extent = extent.max(z - a);
        }
        let scale = if extent == 0. { 1. } else { 2. / extent };
        let frame = Self { origin, scale };
        frame.validate(b.min.len())?;
        Ok(frame)
    }
    fn validate(&self, dimension: usize) -> Result<()> {
        check(
            self.origin.len() == dimension
                && self.origin.iter().all(|x| x.is_finite())
                && self.scale.is_finite()
                && self.scale > 0.,
            "Frame requires matching finite origin and positive finite scale",
        )
    }
    fn point(&self, p: &[f64], to_world: bool) -> Result<Vec<f64>> {
        self.validate(p.len())?;
        Ok(p.iter()
            .zip(&self.origin)
            .map(|(x, o)| {
                if to_world {
                    x / self.scale + o
                } else {
                    (x - o) * self.scale
                }
            })
            .collect())
    }
}
pub fn curve(source: &Curve, frame: &Frame, to_world: bool) -> Result<Curve> {
    source.validate()?;
    let mut result = source.clone();
    result.control_points = source
        .control_points
        .iter()
        .map(|p| frame.point(p, to_world))
        .collect::<Result<_>>()?;
    result.validate()?;
    Ok(result)
}
pub fn surface(source: &Surface, frame: &Frame, to_world: bool) -> Result<Surface> {
    source.validate()?;
    let mut result = source.clone();
    result.control_points = source
        .control_points
        .iter()
        .map(|row| {
            row.iter()
                .map(|p| frame.point(p, to_world))
                .collect::<Result<_>>()
        })
        .collect::<Result<_>>()?;
    result.validate()?;
    Ok(result)
}
/// Explicit unit conversion by a positive finite coordinate multiplier.
pub fn scale_curve(source: &Curve, factor: f64) -> Result<Curve> {
    source.validate()?;
    curve(
        source,
        &Frame {
            origin: vec![0.; source.control_points[0].len()],
            scale: factor,
        },
        false,
    )
}
pub fn scale_surface(source: &Surface, factor: f64) -> Result<Surface> {
    surface(
        source,
        &Frame {
            origin: vec![0.; 3],
            scale: factor,
        },
        false,
    )
}
