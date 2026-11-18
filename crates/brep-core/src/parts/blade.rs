//! Section-authored blades with a symmetric cubic Bezier airfoil.
//! This is a geometric design profile, not a standardized aerodynamic section.
//! Chord, thickness-control height, twist and stacking are authored per station.
//! Global embedding and aerodynamic performance require separate qualification.
use crate::Model;
use nurbs_core::{Error, Result, curve::Curve};

#[derive(Clone, Copy, Debug)]
pub struct Section {
    /// Leading edge position; all sections lie in parallel XY planes.
    pub leading_edge: [f64; 3],
    pub chord: f64,
    /// Separation of the first upper/lower off-axis controls. This is not the
    /// maximum physical thickness; the exact profile is documented below.
    pub thickness_control: f64,
    pub twist_radians: f64,
}
fn invalid(message: &str) -> Error {
    Error::new("BREP_BLADE_INVALID_INPUT", message)
}
/// Lower and upper cubic Bezier arcs, counterclockwise, with a shared leading
/// and trailing edge. In local chord units x(t)=1.95t^2-0.95t^3 and
/// y(t)=+/-h*(1.5t-1.95t^2+0.45t^3). Endpoints are exact shared stored values.
/// h is `thickness_control`; positive chord and h give a simple planar loop.
pub fn profile(section: Section) -> Result<Vec<Curve>> {
    if !section
        .leading_edge
        .iter()
        .all(|x| x.is_finite() && x.abs() <= 1e6)
        || !section.chord.is_finite()
        || !(0. ..=1e6).contains(&section.chord)
        || section.chord == 0.
        || !section.thickness_control.is_finite()
        || section.thickness_control <= 0.
        || section.thickness_control > section.chord
        || !section.twist_radians.is_finite()
        || section.twist_radians.abs() > std::f64::consts::TAU * 1024.
    {
        return Err(invalid(
            "Blade needs finite bounded positions, positive chord/thickness, thickness <= chord and bounded twist",
        ));
    }
    let (sin, cos) = section.twist_radians.sin_cos();
    let point = |x: f64, y: f64| -> Result<Vec<f64>> {
        let p = vec![
            section.leading_edge[0] + cos * x - sin * y,
            section.leading_edge[1] + sin * x + cos * y,
            section.leading_edge[2],
        ];
        if !p.iter().all(|x| x.is_finite() && x.abs() <= 1e6) {
            return Err(invalid("Blade control exceeds coordinate bound"));
        }
        Ok(p)
    };
    let c = section.chord;
    let h = section.thickness_control;
    let lower = Curve {
        degree: 3,
        knots: vec![0., 0., 0., 0., 1., 1., 1., 1.],
        control_points: vec![
            point(0., 0.)?,
            point(0., -0.5 * h)?,
            point(0.65 * c, -0.35 * h)?,
            point(c, 0.)?,
        ],
        weights: vec![1.; 4],
        periodic: false,
    };
    let upper = Curve {
        control_points: vec![
            point(c, 0.)?,
            point(0.65 * c, 0.35 * h)?,
            point(0., 0.5 * h)?,
            point(0., 0.)?,
        ],
        ..lower.clone()
    };
    lower.validate()?;
    upper.validate()?;
    if lower.control_points[0] == lower.control_points[3] {
        return Err(invalid(
            "Blade chord collapsed at the supplied coordinate scale",
        ));
    }
    Ok(vec![lower, upper])
}
/// Capped piecewise ruled blade. All section Z coordinates must strictly
/// increase; 2..129 sections yield at most 514 faces. Each side is an actual
/// rational surface with shared boundary edges, rather than a sampled mesh.
/// The stored model is checked by the existing section-loft/cap audits.
/// Extreme twist/stacking can still self-intersect: combinatorial closure is
/// not a certificate of global solid embedding.
pub fn build(sections: &[Section]) -> Result<Model> {
    if !(2..=129).contains(&sections.len())
        || !sections
            .windows(2)
            .all(|w| w[0].leading_edge[2] < w[1].leading_edge[2])
    {
        return Err(invalid(
            "Blade needs 2..129 sections at strictly increasing Z",
        ));
    }
    let loops = sections
        .iter()
        .map(|s| {
            let mut pieces = Vec::new();
            for curve in profile(*s)? {
                pieces.extend(curve.split(0.5)?);
            }
            Ok(vec![pieces])
        })
        .collect::<Result<Vec<_>>>()?;
    crate::rational_section_loft(&loops)
}
