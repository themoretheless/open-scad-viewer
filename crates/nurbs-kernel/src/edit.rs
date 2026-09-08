//! Native control-space edits preserve rational definitions. A nonlinear edit of
//! controls is not asserted to equal pointwise deformation of the evaluated surface.
use crate::{curve::Curve, surface::Surface, Error, Result};
fn map_curve(
    c: &Curve,
    f: impl Fn([f64; 3]) -> std::result::Result<[f64; 3], String>,
) -> Result<Curve> {
    c.validate()?;
    let mut out = c.clone();
    for p in &mut out.control_points {
        if p.len() != 3 {
            return Err(Error::input("Control edit requires 3D coordinates"));
        }
        *p = f([p[0], p[1], p[2]]).map_err(Error::input)?.to_vec();
    }
    out.validate()?;
    Ok(out)
}
fn map_surface(
    s: &Surface,
    f: impl Fn([f64; 3]) -> std::result::Result<[f64; 3], String>,
) -> Result<Surface> {
    s.validate()?;
    let mut out = s.clone();
    for p in out.control_points.iter_mut().flatten() {
        *p = f([p[0], p[1], p[2]]).map_err(Error::input)?.to_vec();
    }
    out.validate()?;
    Ok(out)
}
pub fn deform_curve(c: &Curve, d: &geometry_ops::Deformation) -> Result<Curve> {
    d.validate().map_err(Error::input)?;
    map_curve(c, |p| d.apply(p))
}
pub fn deform_surface(s: &Surface, d: &geometry_ops::Deformation) -> Result<Surface> {
    d.validate().map_err(Error::input)?;
    map_surface(s, |p| d.apply(p))
}
pub fn brush_curve(c: &Curve, b: &geometry_ops::Brush) -> Result<Curve> {
    b.validate().map_err(Error::input)?;
    map_curve(c, |p| b.apply(p))
}
pub fn brush_surface(s: &Surface, b: &geometry_ops::Brush) -> Result<Surface> {
    b.validate().map_err(Error::input)?;
    map_surface(s, |p| b.apply(p))
}
