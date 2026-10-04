//! Straight-sided unshifted rack outline in XY, as a single closed degree-one NURBS.
use crate::{Result, check, curve::Curve};
#[derive(Clone, Copy, Debug)]
pub struct Spec {
    pub teeth: usize,
    pub module: f64,
    pub pressure_angle_radians: f64,
    /// Reduction of each tooth's pitch-line thickness, in model length units.
    pub backlash: f64,
    pub addendum: f64,
    pub dedendum: f64,
    pub backing_height: f64,
}
impl Default for Spec {
    fn default() -> Self {
        Self {
            teeth: 12,
            module: 1.,
            pressure_angle_radians: 20_f64.to_radians(),
            backlash: 0.,
            addendum: 1.,
            dedendum: 1.25,
            backing_height: 1.,
        }
    }
}
#[derive(Clone, Debug)]
pub struct Profile {
    pub boundary: Curve,
    pub pitch: f64,
    pub width: f64,
    pub tooth_thickness_at_pitch: f64,
    /// Tooth center X coordinates, for placement and mating setup.
    pub tooth_centers: Vec<f64>,
}
/// Bottom starts at x=0, y=-dedendum-backing_height; pitch line is y=0.
/// Boundary is CCW, closed with an exactly repeated first control point.
/// Tooth flanks and top/root lands are straight, with sharp corners. This is
/// the authored rack profile, not a cutter-generated gear or strength report.
pub fn profile(spec: Spec) -> Result<Profile> {
    check((1..=50).contains(&spec.teeth), "Rack requires 1..50 teeth")?;
    check(
        [
            spec.module,
            spec.pressure_angle_radians,
            spec.backlash,
            spec.addendum,
            spec.dedendum,
            spec.backing_height,
        ]
        .iter()
        .all(|v| v.is_finite()),
        "Rack dimensions must be finite",
    )?;
    check(
        spec.module > 0. && spec.addendum > 0. && spec.dedendum > 0. && spec.backing_height > 0.,
        "Rack module, tooth heights and backing height must be positive",
    )?;
    check(
        spec.pressure_angle_radians > 0.
            && spec.pressure_angle_radians < std::f64::consts::FRAC_PI_4,
        "Rack pressure angle must be strictly between zero and 45 degrees",
    )?;
    let pitch = std::f64::consts::PI * spec.module;
    let thickness = pitch / 2. - spec.backlash;
    check(
        spec.backlash >= 0. && thickness > 0.,
        "Rack backlash must leave a positive pitch-line tooth thickness",
    )?;
    let slope = spec.pressure_angle_radians.tan();
    let half_tip = thickness / 2. - spec.addendum * slope;
    let half_root = thickness / 2. + spec.dedendum * slope;
    check(
        half_tip > 0. && half_root < pitch / 2.,
        "Rack tooth tips collapse or adjacent roots overlap",
    )?;
    let width = pitch * spec.teeth as f64;
    let bottom = -spec.dedendum - spec.backing_height;
    check(
        width.is_finite() && bottom.is_finite(),
        "Rack size overflow",
    )?;
    let centers: Vec<_> = (0..spec.teeth).map(|i| (i as f64 + 0.5) * pitch).collect();
    let mut points = vec![
        vec![0., bottom, 0.],
        vec![width, bottom, 0.],
        vec![width, -spec.dedendum, 0.],
    ];
    // Walk the toothed upper boundary from right to left, keeping material left.
    for &center in centers.iter().rev() {
        check(
            center + half_root > center + half_tip && center - half_root < center - half_tip,
            "Rack flank slope collapses at coordinate precision",
        )?;
        points.push(vec![center + half_root, -spec.dedendum, 0.]);
        points.push(vec![center + half_tip, spec.addendum, 0.]);
        points.push(vec![center - half_tip, spec.addendum, 0.]);
        points.push(vec![center - half_root, -spec.dedendum, 0.]);
    }
    points.push(vec![0., -spec.dedendum, 0.]);
    points.push(points[0].clone());
    // Refuse any feature lost to binary64 precision; don't silently remove teeth.
    check(
        points.windows(2).all(|p| p[0] != p[1]),
        "Rack feature collapses at coordinate precision",
    )?;
    let boundary = Curve::from_polyline(points)?;
    Ok(Profile {
        boundary,
        pitch,
        width,
        tooth_thickness_at_pitch: thickness,
        tooth_centers: centers,
    })
}
