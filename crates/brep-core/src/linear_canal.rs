//! Variable-radius sphere-family envelopes on an arbitrary straight 3D spine.
//! Authors NURBS support sheets. Qualification and closed-body admission remain
//! separate: a construction formula cannot authorize its own rounded output.
use crate::{
    Model, Result,
    circular_blend::{CircularBlendBoundary, CircularEnvelopeReport},
    invalid,
};
use nurbs_core::{
    curve::Curve,
    surface::{Axis, Surface},
};
pub struct Span {
    surface: Surface,
    centers: Curve,
    radius: Curve,
    frame: Option<Frame>,
}
struct Frame {
    tangent: [f64; 3],
    angular: [[f64; 3]; 3],
    slope: f64,
    radial_factor: f64,
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
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum End {
    Start,
    Finish,
}
impl Span {
    /// Sphere-family endpoint caps. The characteristic junction row is copied
    /// from the unchanged canal so curve definitions agree exactly.
    /// A zero-radius end has no sphere patch; its original canal pole remains.
    pub fn end_caps(&self, end: End) -> Result<Vec<Span>> {
        let f = self
            .frame
            .as_ref()
            .ok_or_else(|| invalid("Only an authored linear canal has endpoint sphere caps"))?;
        let index = usize::from(end == End::Finish);
        let center = &self.centers.control_points[index];
        let r = self.radius.control_points[index][0];
        if r == 0. {
            return Ok(Vec::new());
        }
        let join = f.slope.acos();
        let (lo, hi) = if index == 0 {
            (0., join)
        } else {
            (join, std::f64::consts::PI)
        };
        let count = ((hi - lo) / std::f64::consts::FRAC_PI_2).ceil() as usize;
        let angles = (0..=count)
            .map(|i| lo + (hi - lo) * i as f64 / count as f64)
            .collect::<Vec<_>>();
        let meridians = angles
            .iter()
            .enumerate()
            .map(|(i, &a)| {
                if (index == 0 && i == count) || (index == 1 && i == 0) {
                    [f.radial_factor, -f.slope]
                } else if a == 0. {
                    [0., -1.]
                } else if a == std::f64::consts::PI {
                    [0., 1.]
                } else {
                    [a.sin(), -a.cos()]
                }
            })
            .collect::<Vec<_>>();
        let c = Curve::from_polyline(vec![center.clone(), center.clone()])?;
        let radius = Curve::from_polyline(vec![vec![r, 0.], vec![r, 0.]])?;
        let mut out = Vec::new();
        for i in 0..count {
            let middle = (angles[i] + angles[i + 1]) * 0.5;
            let w = ((angles[i + 1] - angles[i]) * 0.5).cos();
            let meridian = [
                meridians[i],
                [middle.sin() / w, -middle.cos() / w],
                meridians[i + 1],
            ];
            let mut controls = meridian
                .iter()
                .map(|p| {
                    f.angular
                        .iter()
                        .map(|q| {
                            (0..3)
                                .map(|k| center[k] + r * p[1] * f.tangent[k] + r * p[0] * q[k])
                                .collect()
                        })
                        .collect()
                })
                .collect::<Vec<Vec<Vec<f64>>>>();
            if index == 0 && i + 1 == count {
                controls[2] = self.surface.control_points[0].clone();
            }
            if index == 1 && i == 0 {
                controls[0] = self.surface.control_points[1].clone();
            }
            let weights = [1., w, 1.]
                .iter()
                .map(|&m| self.surface.weights[index].iter().map(|&a| m * a).collect())
                .collect();
            let surface = Surface {
                degree_u: 2,
                degree_v: 2,
                knots_u: vec![0., 0., 0., 1., 1., 1.],
                knots_v: self.surface.knots_v.clone(),
                control_points: controls,
                weights,
                periodic_u: false,
                periodic_v: false,
            };
            surface.validate()?;
            out.push(Span {
                surface,
                centers: c.clone(),
                radius: radius.clone(),
                frame: None,
            });
        }
        Ok(out)
    }
}
/// Assemble the existing support region and its exact authored endpoint caps.
/// Keeps open-sheet status until source pole/embedding/body admission is proven.
pub fn to_capped_region(spans: &[Span], tolerance_mm: f64) -> Result<Model> {
    let mut sheets = Vec::new();
    for span in spans {
        sheets.push((span.to_open_sheet(tolerance_mm)?, false));
        for end in [End::Start, End::Finish] {
            for cap in span.end_caps(end)? {
                sheets.push((cap.to_open_sheet(tolerance_mm)?, false));
            }
        }
    }
    crate::circular_blend::assemble_support_sheets(sheets)
}

/// Recompute qualified original UV regions, ordinary edge ownership and pole
/// contractions for the authored capped supports. No body or embedding claim.
/// Region limits apply per face; exact shell work is shared across all uses.
pub fn to_capped_source_shell(
    spans: &[Span],
    tolerance_mm: f64,
    tolerance_uv: f64,
    region_limits: crate::trimmed_face_recipe::Limits,
    max_exact_work: u64,
) -> Result<crate::source_shell_incidence::Report> {
    use crate::source_shell_incidence::{Address, Pair, Pole};
    use std::collections::BTreeMap;
    if spans.is_empty() || spans.len() > 64 || !(1..=100_000_000).contains(&max_exact_work) {
        return Err(invalid("Choose 1..64 authored canal spans"));
    }
    let model = to_capped_region(spans, tolerance_mm)?;
    let mut regions = Vec::new();
    let mut pairs = Vec::new();
    let mut poles = Vec::new();
    let mut pending = BTreeMap::new();
    for (face, f) in model.faces.iter().enumerate() {
        let mut boundaries = Vec::new();
        for (edge, use_) in model.loops[f.outer].coedges.iter().enumerate() {
            let a = Address {
                face,
                wire: 0,
                edge,
            };
            let world = &model.edges[use_.edge];
            boundaries.push(crate::trimmed_face_recipe::Boundary {
                curve: world.curve.clone(),
                pcurve: use_.pcurve.clone(),
                reversed: use_.reversed,
            });
            if world.degenerate {
                poles.push(Pole {
                    use_: a,
                    point: model.vertices[world.vertices[0]].point,
                });
            } else if let Some((other, reversed)) = pending.remove(&use_.edge) {
                pairs.push(Pair {
                    uses: [other, a],
                    world: world.curve.clone(),
                    world_reversed: [reversed, use_.reversed],
                    cutters: [None, None],
                });
            } else {
                pending.insert(use_.edge, (a, use_.reversed));
            }
        }
        let r = crate::source_contour_proposal::qualify_original_region(
            &cad_predicates::ToleranceContext::default_valid(),
            &f.surface,
            &[boundaries],
            tolerance_uv,
            crate::trimmed_face_recipe::Limits {
                pairs: region_limits.pairs,
                region_cells: region_limits.region_cells,
                domain_cells: region_limits.domain_cells,
                agreement_cells: region_limits.agreement_cells,
            },
        )?;
        let Some(region) = r.region else {
            return Err(invalid(format!("Source face {face}: {}", r.reason)));
        };
        regions.push(region);
    }
    if !pending.is_empty() {
        return Err(invalid(
            "Capped source supports retain unpaired ordinary edges",
        ));
    }
    crate::source_shell_incidence::assemble_regions_with_poles(
        &regions,
        &pairs,
        &poles,
        max_exact_work,
    )
    .map_err(|e| crate::Error::new(e.code, e.message))
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
            frame: Some(Frame {
                tangent,
                angular: unit.map(|p| std::array::from_fn(|k| p[0] * x[k] + p[1] * y[k])),
                slope: a,
                radial_factor: b,
            }),
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
    #[test]
    fn capped_source_embedding_qualifies_every_neighbor_and_non_neighbor_pair() {
        let spans = construct(
            [[10., -7., 5.], [13., -3., 17.]],
            [0.5, 1.25],
            [1., 0., 0.],
            std::f64::consts::TAU,
        )
        .unwrap();
        let shell = to_capped_source_shell(
            &spans,
            1e-7,
            1e-8,
            crate::trimmed_face_recipe::Limits {
                pairs: 10000,
                region_cells: 10000,
                domain_cells: 10000,
                agreement_cells: 10000,
            },
            100_000_000,
        )
        .unwrap()
        .shell
        .unwrap();
        let face_count = shell.faces().len();
        let audit = crate::source_shell_geometry::qualify(
            shell,
            crate::source_shell_geometry::Limits {
                tolerance_uv: 1e-8,
                corners: 10000,
                spans: 20000,
                linear_cells: 10000,
                pairs: crate::face_contacts::Limits {
                    pairs: 10000,
                    cells: 10000,
                    domain_cells: 100000,
                    cells_per_pair: 32,
                    domain_cells_per_pair: 512,
                },
                exact_work: 10000000,
                driver_cells: 10000,
            },
        )
        .unwrap();
        eprintln!(
            "capped source embedding: {} next={:?} pairs={}/{} spans={} driver={} exact={}",
            audit.reason,
            audit.next_pair,
            audit.pairs,
            face_count * (face_count - 1) / 2,
            audit.spans,
            audit.driver_cells,
            audit.exact_work
        );
        assert!(audit.geometry.is_some());
        assert_eq!(audit.reason, "source-shell-embedded-geometry-qualified");
        assert!(audit.next_pair.is_none());
        assert_eq!(audit.pairs, face_count * (face_count - 1) / 2);
    }
    #[test]
    fn capped_source_factory_rejects_open_angular_boundaries_and_invalid_work() {
        let spans = construct([[0.; 3], [0., 0., 8.]], [0.5, 1.], [1., 0., 0.], 0.7).unwrap();
        let limits = || crate::trimmed_face_recipe::Limits {
            pairs: 10000,
            region_cells: 10000,
            domain_cells: 10000,
            agreement_cells: 10000,
        };
        assert!(to_capped_source_shell(&spans, 1e-7, 1e-8, limits(), 1000000).is_err());
        assert!(to_capped_source_shell(&spans, 1e-7, 1e-8, limits(), 0).is_err());
        assert!(to_capped_source_shell(&[], 1e-7, 1e-8, limits(), 1000000).is_err());
    }
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
    #[test]
    fn sphere_endcaps_preserve_junction_curves_and_certify_tangent_planes() {
        use crate::source_boundary_fragment::{Endpoint, Fragment};
        for radii in [[0.5, 1.25], [1.25, 0.5]] {
            let span = construct([[10., -7., 5.], [13., -3., 17.]], radii, [1., 0., 0.], 0.7)
                .unwrap()
                .remove(0);
            for end in [End::Start, End::Finish] {
                let caps = span.end_caps(end).unwrap();
                assert!(!caps.is_empty());
                let index = usize::from(end == End::Finish);
                let cap = if index == 0 {
                    caps.last().unwrap()
                } else {
                    &caps[0]
                };
                let cap_u = if index == 0 { 1. } else { 0. };
                let world = span.surface().iso(Axis::U, index as f64).unwrap();
                assert_eq!(world, cap.surface().iso(Axis::U, cap_u).unwrap());
                let a = Curve::from_polyline(vec![vec![index as f64, 0.], vec![index as f64, 1.]])
                    .unwrap();
                let b = Curve::from_polyline(vec![vec![cap_u, 0.], vec![cap_u, 1.]]).unwrap();
                let a = Fragment::new(
                    span.surface(),
                    &a,
                    Endpoint::Parameter(0.),
                    Endpoint::Parameter(1.),
                )
                .unwrap();
                let b = Fragment::new(
                    cap.surface(),
                    &b,
                    Endpoint::Parameter(1.),
                    Endpoint::Parameter(0.),
                )
                .unwrap();
                let edge = crate::source_shared_edge::qualify(
                    &world,
                    [&a, &b],
                    [false, false],
                    100_000_000,
                )
                .unwrap()
                .edge
                .unwrap();
                let tangent = crate::source_seam_tangency::qualify(
                    &edge,
                    crate::source_seam_tangency::Limits {
                        max_sine_squared: 1e-3,
                        cells: 100000,
                        curve_spans: 100000,
                        normal_spans: 100000,
                    },
                )
                .unwrap();
                eprintln!(
                    "cap {:?} radii {:?}: {} cells {} accepted {}",
                    end, radii, tangent.reason, tangent.cells, tangent.accepted_cells
                );
                assert!(
                    tangent.seam.is_some(),
                    "{} {:?}",
                    tangent.reason,
                    tangent.uncertain_canonical
                );
                for cap in &caps {
                    let radius = nurbs_core::moving_radius::qualify(
                        cap.surface(),
                        cap.centers(),
                        cap.radius(),
                        1e-8,
                        100,
                        100_000_000,
                    )
                    .unwrap();
                    assert!(
                        radius.certificate.is_some(),
                        "{} {:?}",
                        radius.reason,
                        radius.error_upper
                    );
                    for boundary in cap.boundaries().unwrap() {
                        if let Some(point) = boundary.collapsed_pole {
                            let source = Fragment::new(
                                cap.surface(),
                                &boundary.pcurve,
                                Endpoint::Parameter(0.),
                                Endpoint::Parameter(1.),
                            )
                            .unwrap();
                            let pole = crate::source_collapsed_boundary::qualify(
                                &source,
                                point,
                                cad_predicates::MAX_WORK,
                            )
                            .unwrap();
                            assert!(pole.boundary.is_some(), "{}", pole.reason);
                        }
                    }
                    cap.to_open_sheet(1e-7).unwrap().validate().unwrap();
                }
            }
        }
    }
    #[test]
    fn capped_full_turn_has_poles_and_paired_nondegenerate_edges_without_body_claim() {
        for radii in [[0.5, 1.25], [1.25, 0.5], [0., 1.], [1., 0.]] {
            let spans = construct(
                [[10., -7., 5.], [13., -3., 17.]],
                radii,
                [1., 0., 0.],
                std::f64::consts::TAU,
            )
            .unwrap();
            let model = to_capped_region(&spans, 1e-7).unwrap();
            model.validate().unwrap();
            let mut uses = vec![Vec::new(); model.edges.len()];
            for loop_ in &model.loops {
                for coedge in &loop_.coedges {
                    uses[coedge.edge].push(coedge.reversed);
                }
            }
            for (i, edge) in model.edges.iter().enumerate() {
                if edge.degenerate {
                    assert_eq!(uses[i].len(), 1);
                } else {
                    assert_eq!(uses[i].len(), 2);
                    assert_ne!(uses[i][0], uses[i][1]);
                }
            }
            assert!(model.bodies.is_empty());
            assert!(!model.shells[0].closed);
            // Pole boundary identities follow their source face/UV ownership,
            // not face array order or arbitrary duplicate numbering.
            let ids = model
                .1
                .edges
                .iter()
                .copied()
                .collect::<std::collections::BTreeSet<_>>();
            let mut permuted = model.clone();
            let count = permuted.faces.len();
            permuted.0.faces.reverse();
            for shell in &mut permuted.0.shells {
                for usage in &mut shell.faces {
                    usage.face = count - 1 - usage.face;
                }
            }
            permuted.rebuild_topology_ids();
            permuted.validate().unwrap();
            assert_eq!(ids, permuted.1.edges.iter().copied().collect());
            let saved = value_codec::to_string(&model).unwrap();
            let restored: Model = value_codec::from_str(&saved).unwrap();
            restored.validate().unwrap();
            assert_eq!(restored, model);
        }
    }
}
