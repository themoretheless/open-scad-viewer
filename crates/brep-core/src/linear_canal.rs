//! Variable-radius sphere-family envelopes on an arbitrary straight 3D spine.
//! Authors NURBS support sheets. Qualification and closed-body admission remain
//! separate: a construction formula cannot authorize its own rounded output.
use crate::{
    circular_blend::{CircularBlendBoundary, CircularEnvelopeReport},
    invalid, Model, Result,
};
use nurbs_core::{
    curve::Curve,
    surface::{Axis, Surface},
};
pub struct Span {
    surface: Surface,
    centers: Curve,
    radius: Curve,
}
impl Span {
    pub fn surface(&self) -> &Surface {
        &self.surface
    }
    pub fn centers(&self) -> &Curve {
        &self.centers
    }
    pub fn radius(&self) -> &Curve {
        &self.radius
    }
    pub fn qualify(
        &self,
        tolerance_mm: f64,
        radius_cells: usize,
        radius_work: u64,
        limits: nurbs_core::moving_envelope::Limits,
    ) -> nurbs_core::Result<CircularEnvelopeReport> {
        let mut radius = nurbs_core::moving_radius::qualify(
            &self.surface,
            &self.centers,
            &self.radius,
            tolerance_mm,
            radius_cells,
            radius_work,
        )?;
        let normals = radius
            .certificate
            .take()
            .map(|proof| nurbs_core::moving_envelope::qualify(proof, limits))
            .transpose()?;
        Ok(CircularEnvelopeReport { radius, normals })
    }
    /// Directed authored natural-chart boundaries, including a retained pole.
    pub fn boundaries(&self) -> Result<[CircularBlendBoundary; 4]> {
        let curves = [
            self.surface.iso(Axis::V, 0.)?,
            self.surface.iso(Axis::U, 1.)?,
            self.surface.iso(Axis::V, 1.)?.reverse()?,
            self.surface.iso(Axis::U, 0.)?.reverse()?,
        ];
        let corners = [[0., 0.], [1., 0.], [1., 1.], [0., 1.]];
        let mut out = Vec::new();
        for (i, curve) in curves.into_iter().enumerate() {
            let pcurve =
                Curve::from_polyline(vec![corners[i].to_vec(), corners[(i + 1) % 4].to_vec()])?;
            let first = &curve.control_points[0];
            let collapsed_pole = curve
                .control_points
                .iter()
                .all(|p| p == first)
                .then(|| [first[0], first[1], first[2]]);
            if let Some(pole) = collapsed_pole {
                crate::validate_pole_boundary(&self.surface, &pcurve, pole)?;
            }
            out.push(CircularBlendBoundary {
                curve,
                pcurve,
                collapsed_pole,
            });
        }
        match out.try_into() {
            Ok(a) => Ok(a),
            Err(_) => unreachable!("four authored boundaries"),
        }
    }
    pub fn to_open_sheet(&self, tolerance_mm: f64) -> Result<Model> {
        crate::circular_blend::open_face(self.surface.clone(), self.boundaries()?, tolerance_mm)
    }
}
fn dot(a: [f64; 3], b: [f64; 3]) -> f64 {
    (0..3).map(|k| a[k] * b[k]).sum()
}
fn length(a: [f64; 3]) -> f64 {
    a[0].hypot(a[1]).hypot(a[2])
}
fn cross(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    std::array::from_fn(|k| a[(k + 1) % 3] * b[(k + 2) % 3] - a[(k + 2) % 3] * b[(k + 1) % 3])
}
/// For speed h=|C1-C0| and a=(r1-r0)/h, the sphere envelope circle
/// has center C-r*a*T and radius r*sqrt(1-a^2). Thus it accounts for
/// radius derivative instead of placing an ordinary radius-r circle at C.
/// Input direction fixes the angular start after projection onto the spine's
/// normal plane. Patches share exactly the same authored endpoint directions.
/// Zero-radius ends retain poles and do not receive regular envelope authority.
pub fn construct(
    centers: [[f64; 3]; 2],
    radii: [f64; 2],
    direction: [f64; 3],
    sweep: f64,
) -> Result<Vec<Span>> {
    if !centers
        .iter()
        .flatten()
        .chain(&radii)
        .chain(&direction)
        .all(|x| x.is_finite())
        || radii.iter().any(|&r| r < 0.)
        || radii == [0., 0.]
        || !sweep.is_finite()
        || sweep.abs() < 1e-12
        || sweep.abs() > std::f64::consts::TAU
    {
        return Err(invalid(
            "Choose finite spine/radii/direction and a nonzero arc of at most one turn",
        ));
    }
    let d = std::array::from_fn(|k| centers[1][k] - centers[0][k]);
    let h = length(d);
    let delta = radii[1] - radii[0];
    if !h.is_finite() || h <= 0. || delta.abs() >= h {
        return Err(invalid(
            "Radius slope must have magnitude less than the spine speed",
        ));
    }
    let tangent = d.map(|x| x / h);
    let along = dot(direction, tangent);
    let radial = std::array::from_fn(|k| direction[k] - along * tangent[k]);
    let l = length(radial);
    if !l.is_finite() || l <= length(direction) * 1e-12 {
        return Err(invalid(
            "Angular direction must have a resolved component normal to the spine",
        ));
    }
    let x = radial.map(|a| a / l);
    let y = cross(tangent, x);
    let a = delta / h;
    let b = ((1. - a) * (1. + a)).sqrt();
    let count = (sweep.abs() / std::f64::consts::FRAC_PI_2).ceil() as usize;
    let angles = (0..=count)
        .map(|i| sweep * i as f64 / count as f64)
        .collect::<Vec<_>>();
    let ends = angles
        .iter()
        .map(|&t| crate::circular_blend::angular_unit(t))
        .collect::<Vec<_>>();
    let c = Curve::from_polyline(centers.iter().map(|p| p.to_vec()).collect())?;
    let r = Curve::from_polyline(radii.iter().map(|&r| vec![r, 0.]).collect())?;
    let mut out = Vec::new();
    for i in 0..count {
        let middle = (angles[i] + angles[i + 1]) * 0.5;
        let w = ((angles[i + 1] - angles[i]) * 0.5).cos();
        let unit = [ends[i], [middle.cos() / w, middle.sin() / w], ends[i + 1]];
        let controls = (0..2)
            .map(|u| {
                unit.iter()
                    .map(|p| {
                        (0..3)
                            .map(|k| {
                                centers[u][k] - radii[u] * a * tangent[k]
                                    + radii[u] * b * (p[0] * x[k] + p[1] * y[k])
                            })
                            .collect()
                    })
                    .collect()
            })
            .collect();
        let surface = Surface {
            degree_u: 1,
            degree_v: 2,
            knots_u: vec![0., 0., 1., 1.],
            knots_v: vec![0., 0., 0., 1., 1., 1.],
            control_points: controls,
            weights: vec![vec![1., w, 1.]; 2],
            periodic_u: false,
            periodic_v: false,
        };
        surface.validate()?;
        out.push(Span {
            surface,
            centers: c.clone(),
            radius: r.clone(),
        });
    }
    Ok(out)
}
/// Assemble only exact authored shared rails into the existing open-sheet Model.
/// No positional tolerance welding, end caps or closed-body status is introduced.
pub fn to_open_region(spans: &[Span], tolerance_mm: f64) -> Result<Model> {
    crate::circular_blend::assemble_support_sheets(
        spans
            .iter()
            .map(|s| s.to_open_sheet(tolerance_mm).map(|m| (m, false)))
            .collect::<Result<Vec<_>>>()?,
    )
}
#[cfg(test)]
mod tests {
    use super::*;
    fn limits() -> nurbs_core::moving_envelope::Limits {
        nurbs_core::moving_envelope::Limits {
            max_sine_squared: 0.01,
            cells: 100000,
            surface_spans: 100000,
            center_spans: 100000,
            radial_work: 100_000_000,
        }
    }
    #[test]
    fn increasing_and_decreasing_radius_are_envelopes_on_rotated_translated_spines() {
        for radii in [[0.5, 1.25], [1.25, 0.5]] {
            for sweep in [-0.7, 0.7] {
                let spans = construct(
                    [[10., -7., 5.], [13., -3., 17.]],
                    radii,
                    [1., 0., 0.],
                    sweep,
                )
                .unwrap();
                assert_eq!(spans.len(), 1);
                let span = &spans[0];
                let before = span.surface().clone();
                let report = span.qualify(1e-8, 100, 100_000_000, limits()).unwrap();
                let normal = report.normals.unwrap();
                eprintln!(
                    "line canal {:?} sweep {sweep}: radius error {:?}; {} cells {} accepted {}",
                    radii,
                    report.radius.error_upper,
                    normal.reason,
                    normal.cells,
                    normal.accepted_cells
                );
                assert!(
                    normal.envelope.is_some(),
                    "{} {:?}",
                    normal.reason,
                    normal.uncertain_uv
                );
                let proof = normal.envelope.unwrap();
                assert_eq!(proof.radius().surface(), &before);
                for u in [0., 0.3, 0.5, 0.8, 1.] {
                    for v in [0., 0.4, 1.] {
                        let p = span.surface().evaluate(u, v).unwrap();
                        let c = span.centers().evaluate(u).unwrap().point;
                        let radial = std::array::from_fn(|k| p.point[k] - c[k]);
                        let r = radii[0] + u * (radii[1] - radii[0]);
                        assert!((length(radial) - r).abs() <= proof.radius().error_upper());
                        let (du, dv) = p.first_derivatives().unwrap();
                        assert!(dot(radial, du).abs() < 1e-10);
                        assert!(dot(radial, dv).abs() < 1e-10);
                    }
                }
                let sheet = span.to_open_sheet(1e-7).unwrap();
                sheet.validate().unwrap();
                assert!(sheet.bodies.is_empty());
                assert!(!sheet.shells[0].closed);
                let boundaries = span.boundaries().unwrap();
                for b in boundaries {
                    let r = nurbs_core::curve_surface_agreement::verify_exact_algebraic(
                        &b.curve,
                        &b.pcurve,
                        span.surface(),
                        false,
                        cad_predicates::MAX_WORK,
                    )
                    .unwrap()
                    .unwrap();
                    assert_eq!(r.outcome, cad_predicates::BezierIdentity::Equal);
                }
            }
        }
    }
    #[test]
    fn full_turn_and_partial_arcs_share_exact_authored_rails() {
        for sweep in [-std::f64::consts::TAU, std::f64::consts::TAU, 4.7, -4.7] {
            let spans =
                construct([[0., 0., 0.], [3., 4., 12.]], [1., 2.], [1., 0., 0.], sweep).unwrap();
            for pair in spans.windows(2) {
                assert_eq!(
                    pair[0].surface().iso(Axis::V, 1.).unwrap(),
                    pair[1].surface().iso(Axis::V, 0.).unwrap()
                );
                use crate::source_boundary_fragment::{Endpoint, Fragment};
                let pc_a = Curve::from_polyline(vec![vec![0., 1.], vec![1., 1.]]).unwrap();
                let pc_b = Curve::from_polyline(vec![vec![0., 0.], vec![1., 0.]]).unwrap();
                let a = Fragment::new(
                    pair[0].surface(),
                    &pc_a,
                    Endpoint::Parameter(0.),
                    Endpoint::Parameter(1.),
                )
                .unwrap();
                let b = Fragment::new(
                    pair[1].surface(),
                    &pc_b,
                    Endpoint::Parameter(1.),
                    Endpoint::Parameter(0.),
                )
                .unwrap();
                let world = pair[0].surface().iso(Axis::V, 1.).unwrap();
                let joined = crate::source_shared_edge::qualify(
                    &world,
                    [&a, &b],
                    [false, false],
                    100_000_000,
                )
                .unwrap();
                assert!(joined.edge.is_some(), "{}", joined.reason);
            }
            let model = to_open_region(&spans, 1e-7).unwrap();
            model.validate().unwrap();
            let saved = value_codec::to_string(&model).unwrap();
            let restored: Model = value_codec::from_str(&saved).unwrap();
            restored.validate().unwrap();
            assert_eq!(restored, model);
            assert_eq!(model.faces.len(), spans.len());
            assert!(model.bodies.is_empty());
            assert!(!model.shells[0].closed);
        }
    }
    #[test]
    fn radius_pole_is_retained_and_never_admitted_as_regular_envelope() {
        for radii in [[0., 1.], [1., 0.]] {
            let span = construct([[0., 0., 0.], [0., 0., 5.]], radii, [1., 0., 0.], 0.7)
                .unwrap()
                .remove(0);
            let boundaries = span.boundaries().unwrap();
            assert_eq!(
                boundaries
                    .iter()
                    .filter(|b| b.collapsed_pole.is_some())
                    .count(),
                1
            );
            let sheet = span.to_open_sheet(1e-7).unwrap();
            sheet.validate().unwrap();
            assert_eq!(sheet.vertices.len(), 3);
            let mut l = limits();
            l.cells = 32;
            let report = span.qualify(1e-6, 100, 100_000_000, l).unwrap();
            assert_eq!(report.radius.reason, "moving-radius-qualified");
            assert!(report.normals.unwrap().envelope.is_none());
        }
    }
    #[test]
    fn steep_laws_and_unresolved_angular_frames_are_rejected() {
        assert!(construct([[0., 0., 0.], [0., 0., 1.]], [1., 2.], [1., 0., 0.], 0.7).is_err());
        assert!(construct([[0., 0., 0.], [0., 0., 1.]], [1., 3.], [1., 0., 0.], 0.7).is_err());
        assert!(construct([[0., 0., 0.], [0., 0., 1.]], [1., 1.], [0., 0., 1.], 0.7).is_err());
        assert!(construct([[0., 0., 0.]; 2], [1., 1.], [1., 0., 0.], 0.7).is_err());
        assert!(construct([[0., 0., 0.], [0., 0., 5.]], [0., 0.], [1., 0., 0.], 0.7).is_err());
    }
}
