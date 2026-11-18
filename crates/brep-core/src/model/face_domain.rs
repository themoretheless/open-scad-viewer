//! Prepare the authored UV loops of a B-rep face for conservative classification.
//! This does not certify surface/edge agreement or validity of the whole solid.
use crate::{Error, Loop, Model, Result, Surface};
use crate::profile::pcurve_trim::active_pcurves;
use nurbs_core::curve::Curve;
use nurbs_core::trim_domain::{Classification, TrimDomain};

mod rectangle_closure;
mod closed_carrier;
mod box_containment;
mod affine_trim_hull;
mod periodic_wire;
mod paired_chart_seam;
pub(crate) use affine_trim_hull::affine_support;
pub use box_containment::{BoxDecision,BoxContainment,inspect_box_containment};

fn wire_pcurves(surface: &Surface, wire: &Loop) -> Result<Vec<Curve>> {
    let mut curves = Vec::new();
    for coedge in &wire.coedges {
        let active = active_pcurves(coedge)?;
        if let Some(trim) = coedge.trim() {
            periodic_wire::verify_lifts(surface, &active, trim.periodic_lift)?;
        }
        curves.extend(active);
    }
    Ok(curves)
}

fn face_pcurves(
    model: &Model,
    face: &crate::Face,
    complete_contours: bool,
) -> Result<(Vec<Vec<Curve>>, bool)> {
    let mut loops = Vec::new();
    let mut explicit = false;
    for index in std::iter::once(face.outer).chain(face.holes.iter().copied()) {
        let wire = model.loops.get(index).ok_or_else(|| {
            Error::new("BREP_INVALID_TOPOLOGY", "Face references a missing UV loop")
        })?;
        explicit |= wire.coedges.iter().any(|c| c.trim().is_some());
        loops.push(if complete_contours {
            wire.coedges.iter().map(|coedge| coedge.pcurve.clone()).collect()
        } else {
            wire_pcurves(&face.surface, wire)?
        });
    }
    let periodic = explicit && (face.surface.periodic_u || face.surface.periodic_v);
    if periodic {
        paired_chart_seam::open(model, face, &mut loops)?;
    }
    Ok((loops, periodic))
}

pub struct FaceDomain {
    pub(crate) region: TrimDomain,
    natural: [[f64; 2]; 2],
    linear_boundaries: Vec<[[f64; 2]; 2]>,
    rectangle_closure: Option<[[f64; 2]; 2]>,
}
impl FaceDomain {
    pub(crate) fn contains_natural_rectangle(&self, r: [[f64; 2]; 2]) -> bool {
        (0..2).all(|k| {
            r[k][0].is_finite()
                && r[k][1].is_finite()
                && self.natural[k][0] <= r[k][0]
                && r[k][0] <= r[k][1]
                && r[k][1] <= self.natural[k][1]
        })
    }
    pub(crate) fn literal_rectangle(&self) -> Option<[[f64; 2]; 2]> {
        self.rectangle_closure
            .filter(|r| self.contains_closed_rectangle(*r))
    }
    pub fn new(model: &Model, face_index: usize, tolerance_uv: f64) -> Result<Self> {
        let face = model.faces.get(face_index).ok_or_else(|| {
            Error::new(
                "BREP_INVALID_INPUT",
                "Choose an existing face for UV classification",
            )
        })?;
        face.surface.validate()?;
        let (loops, periodic) = face_pcurves(model, face, false)?;
        let (loops, clipped) = if periodic {
            periodic_wire::project(&face.surface, &loops)?
        } else {
            (loops, periodic_wire::Projection::Translated)
        };
        let mut domain = Self::from_pcurves(&face.surface, &loops, tolerance_uv)?;
        if clipped != periodic_wire::Projection::Translated {
            // Chart cuts are classification boundaries. They do not authorize
            // contacts as authored BRep edges.
            let natural = domain.natural;
            domain.linear_boundaries.retain(|segment| {
                ![face.surface.periodic_u, face.surface.periodic_v].into_iter().enumerate().any(|(axis, periodic)| {
                    periodic && natural[axis].iter().any(|&x| segment.iter().all(|p| p[axis] == x))
                })
            });
        }
        Ok(domain)
    }

    pub(crate) fn from_pcurves(
        s: &crate::Surface,
        loops: &[Vec<nurbs_core::curve::Curve>],
        tolerance_uv: f64,
    ) -> Result<Self> {
        let region = TrimDomain::new(loops, tolerance_uv)?;
        let natural = [
            [s.knots_u[s.degree_u], s.knots_u[s.control_points.len()]],
            [s.knots_v[s.degree_v], s.knots_v[s.control_points[0].len()]],
        ];
        let linear_boundaries = loops
            .iter()
            .flatten()
            .filter(|c| c.degree == 1 && c.control_points.iter().all(|p| p.len() == 2))
            .flat_map(|c| {
                c.control_points
                    .windows(2)
                    .map(|p| [[p[0][0], p[0][1]], [p[1][0], p[1][1]]])
            })
            .collect();
        let rectangle_closure = if loops.len() == 1 {
            rectangle_closure::inspect(&loops[0])
        } else {
            None
        };
        Ok(Self {
            rectangle_closure,
            linear_boundaries,
            region,
            natural,
        })
    }
    /// Exact closed-set membership for an authored, possibly subdivided rectangular
    /// outer loop without holes. This does not enlarge a general trim band.
    pub fn contains_closed_rectangle(&self, rectangle: [[f64; 2]; 2]) -> bool {
        self.rectangle_closure.is_some_and(|bounds| {
            (0..2).all(|k| {
                rectangle[k][0].is_finite()
                    && rectangle[k][1].is_finite()
                    && rectangle[k][0] <= rectangle[k][1]
                    && rectangle[k][0] >= bounds[k][0]
                    && rectangle[k][1] <= bounds[k][1]
                    && rectangle[k][0] >= self.natural[k][0]
                    && rectangle[k][1] <= self.natural[k][1]
            })
        })
    }
    /// Membership in the closed authored face boundary, for a degenerate UV
    /// rectangle lying wholly on one axis-aligned degree-one pcurve segment.
    /// Unlike a Boundary-band classification this establishes every point of
    /// the query belongs to the face closure; no tolerance enlargement is used.
    pub fn on_linear_boundary(&self, rectangle: [[f64; 2]; 2]) -> bool {
        if !(0..2).all(|k| {
            rectangle[k][0].is_finite()
                && rectangle[k][1].is_finite()
                && rectangle[k][0] <= rectangle[k][1]
                && rectangle[k][0] >= self.natural[k][0]
                && rectangle[k][1] <= self.natural[k][1]
        }) {
            return false;
        }
        self.linear_boundaries.iter().any(|segment| {
            (0..2).any(|axis| {
                let free = 1 - axis;
                rectangle[axis][0] == rectangle[axis][1]
                    && segment[0][axis] == rectangle[axis][0]
                    && segment[1][axis] == rectangle[axis][0]
                    && rectangle[free][0] >= segment[0][free].min(segment[1][free])
                    && rectangle[free][1] <= segment[0][free].max(segment[1][free])
            })
        })
    }
    pub fn classify(&self, rectangle: [[f64; 2]; 2], max_cells: usize) -> Result<Classification> {
        if !(0..2)
            .all(|k| rectangle[k][0] >= self.natural[k][0] && rectangle[k][1] <= self.natural[k][1])
        {
            return Err(Error::new(
                "BREP_INVALID_INPUT",
                "UV rectangle must lie in the face surface domain",
            ));
        }
        self.region.classify(rectangle, max_cells)
    }
}

/// Tolerances and shared cell budgets of one face or face-set distance query.
#[derive(Clone, Copy, Debug)]
pub struct DistanceLimits {
    pub tolerance_mm: f64,
    pub tolerance_uv: f64,
    pub max_cells: usize,
    pub max_domain_cells: usize,
}
/// Distance between two selected trimmed faces, not between filled body volumes.
pub fn distance_between_faces(
    a: &Model,
    face_a: usize,
    b: &Model,
    face_b: usize,
    limits: DistanceLimits,
) -> Result<nurbs_core::trimmed_surface_distance::TrimmedDistance> {
    let DistanceLimits {
        tolerance_mm,
        tolerance_uv,
        max_cells,
        max_domain_cells,
    } = limits;
    a.validate()?;
    b.validate()?;
    let da = FaceDomain::new(a, face_a, tolerance_uv)?;
    let db = FaceDomain::new(b, face_b, tolerance_uv)?;
    nurbs_core::trimmed_surface_distance::distance(
        &a.faces[face_a].surface,
        &da.region,
        &b.faces[face_b].surface,
        &db.region,
        tolerance_mm,
        max_cells,
        max_domain_cells,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use nurbs_core::{curve::Curve, trim_domain::Location};
    #[test]
    fn restricted_periodic_wire_uses_exact_seam_lifts() {
        let mut model = crate::cuboid([0.; 3], [1.; 3]).unwrap();
        let profile = [[0., 0.], [1., 0.], [1., 1.], [0., 1.], [0., 0.]];
        model.faces[1].surface = Surface {
            degree_u: 1, degree_v: 1,
            knots_u: vec![-1.5, -1., -0.5, 0., 0.5, 1., 1.5],
            knots_v: vec![0., 0., 1., 1.],
            control_points: profile.iter().map(|p| {
                vec![vec![p[0], p[1], 0.], vec![p[0], p[1], 1.]]
            }).collect(),
            weights: vec![vec![1.; 2]; 5],
            periodic_u: true, periodic_v: false,
        };
        let outer = model.faces[1].outer;
        let coedges = &mut model.loops[outer].coedges;
        for coedge in coedges.iter_mut() {
            for p in &mut coedge.pcurve.control_points {
                p[1] = 0.25 + 0.5 * p[1];
            }
        }
        let seam = &mut coedges[1];
        let curve = crate::line(vec![1., 0.], vec![1., 1.]).insert(0.5, 1).unwrap();
        let trim = brep_topology::CoedgeTrim {
            curve_parameter: [0., 1.],
            pcurve_parameter: [0.25, 0.75],
            periodic_lift: [[1, 0]; 2],
        };
        *seam = crate::Coedge::new(seam.edge, seam.reversed, curve).with_trim(trim).unwrap();
        let hole = model.loops.len();
        let corners = [[0.625, 0.375], [0.625, 0.625], [0.875, 0.625], [0.875, 0.375]];
        model.loops.push(Loop {
            coedges: (0..4).map(|i| crate::Coedge::new(
                i, false, crate::line(corners[i].to_vec(), corners[(i + 1) % 4].to_vec()),
            )).collect(),
        });
        model.faces[1].holes.push(hole);
        for period in [-1, 0, 1] {
            let mut shifted = model.clone();
            for wire in [outer, hole] {
                for coedge in &mut shifted.loops[wire].coedges {
                    for p in &mut coedge.pcurve.control_points {
                        p[0] += 2. * f64::from(period);
                    }
                }
            }
            let seam = &mut shifted.loops[outer].coedges[1];
            *seam = seam.clone().with_trim(brep_topology::CoedgeTrim {
                periodic_lift: [[period + 1, 0]; 2], ..trim
            }).unwrap();
            let domain = FaceDomain::new(&shifted, 1, 1e-8).unwrap();
            assert_eq!(domain.classify([[0.5; 2], [0.5; 2]], 1_000).unwrap().location, Location::Inside);
            assert_eq!(domain.classify([[0.5; 2], [0.125; 2]], 1_000).unwrap().location, Location::Outside);
            assert_eq!(domain.classify([[0.75; 2], [0.5; 2]], 1_000).unwrap().location, Location::Outside);
            assert!(domain.on_linear_boundary([[1.; 2], [0.25, 0.5]]));
        }
        let mut straddling = model.clone();
        for wire in [outer, hole] {
            for coedge in &mut straddling.loops[wire].coedges {
                for p in &mut coedge.pcurve.control_points {
                    p[0] += 0.5;
                }
            }
        }
        let domain = FaceDomain::new(&straddling, 1, 1e-8).unwrap();
        for (u, expected) in [
            (0.75, Location::Inside), (-0.5625, Location::Inside),
            (-0.75, Location::Outside), (0., Location::Outside),
        ] {
            assert_eq!(domain.classify([[u; 2], [0.5; 2]], 1_000).unwrap().location, expected);
        }
        assert!(!domain.on_linear_boundary([[1.; 2], [0.25, 0.5]]));
        let top = &mut straddling.loops[outer].coedges[2];
        top.pcurve = Curve {
            degree: 2,
            knots: vec![0., 0., 0., 1., 1., 1.],
            control_points: vec![vec![1.5, 0.75], vec![1., 0.875], vec![0.5, 0.75]],
            weights: vec![1., 3., 1.], periodic: false,
        };
        let curved = FaceDomain::new(&straddling, 1, 1e-8).unwrap();
        for u in [0.9375, -0.9375] {
            assert_eq!(curved.classify([[u; 2], [0.78125; 2]], 10_000).unwrap().location, Location::Inside);
        }
        assert_eq!(curved.classify([[-0.75; 2], [0.5; 2]], 1_000).unwrap().location, Location::Outside);
        assert!(!curved.on_linear_boundary([[1.; 2], [0.25, 0.5]]));
        let make_wide = |points: [[f64; 2]; 4]| {
            let mut wide = model.clone();
            wide.faces[1].holes.clear();
            for (i, coedge) in wide.loops[outer].coedges.iter_mut().enumerate() {
                *coedge = crate::Coedge::new(
                    coedge.edge, coedge.reversed,
                    crate::line(points[i].to_vec(), points[(i + 1) % 4].to_vec()),
                );
            }
            let first = &mut wide.loops[outer].coedges[0];
            *first = first.clone().with_trim(brep_topology::CoedgeTrim {
                curve_parameter: [0., 1.], pcurve_parameter: [0., 1.],
                periodic_lift: [[0, 0], [((points[1][0] + 1.) / 2.).floor() as i32, 0]],
            }).unwrap();
            wide
        };
        let wide = make_wide([[0., 0.125], [5., 0.75], [5., 0.8125], [0., 0.1875]]);
        let domain = FaceDomain::new(&wide, 1, 1e-8).unwrap();
        for v in [0.21875, 0.46875, 0.71875] {
            assert_eq!(domain.classify([[0.5; 2], [v; 2]], 1_000).unwrap().location, Location::Inside);
        }
        assert_eq!(domain.classify([[0.5; 2], [0.34375; 2]], 1_000).unwrap().location, Location::Outside);
        let overlapping = make_wide([[0., 0.125], [5., 0.125], [5., 0.625], [0., 0.625]]);
        let error = FaceDomain::new(&overlapping, 1, 1e-8).err().unwrap();
        assert!(error.message.contains("overlap in their interiors"));
        let mut curved_wide = make_wide([[0., 0.125], [4., 0.625], [4., 0.6875], [0., 0.1875]]);
        for (i, middle) in [(0, 0.4375), (2, 0.5)] {
            let coedge = &mut curved_wide.loops[outer].coedges[i];
            coedge.pcurve.degree = 2;
            coedge.pcurve.control_points.insert(1, vec![2., middle]);
            coedge.pcurve.weights = vec![1.; 3];
            coedge.pcurve.knots = vec![0., 0., 0., 1., 1., 1.];
        }
        let domain = FaceDomain::new(&curved_wide, 1, 1e-8).unwrap();
        for v in [0.21875, 0.46875] {
            assert_eq!(domain.classify([[0.5; 2], [v; 2]], 10_000).unwrap().location, Location::Inside);
        }
        assert_eq!(domain.classify([[0.5; 2], [0.34375; 2]], 10_000).unwrap().location, Location::Outside);
        let mut belt = model.clone();
        let bottom = crate::line(vec![-1., 0.25], vec![1., 0.25]);
        belt.loops[outer].coedges = vec![crate::Coedge::new(0, false, bottom).with_trim(
            brep_topology::CoedgeTrim {
                curve_parameter: [0., 1.], pcurve_parameter: [0., 1.],
                periodic_lift: [[0, 0], [1, 0]],
            },
        ).unwrap()];
        let cap = Curve {
            degree: 2, knots: vec![0., 0., 0., 1., 1., 1.],
            control_points: vec![vec![3., 0.75], vec![2., 0.875], vec![1., 0.75]],
            weights: vec![1.; 3], periodic: false,
        };
        belt.loops[hole].coedges = vec![crate::Coedge::new(1, false, cap)];
        let cutout = belt.loops.len();
        let corners = [[2.25, 0.375], [2.25, 0.625], [2.5, 0.625], [2.5, 0.375]];
        belt.loops.push(Loop {
            coedges: (0..4).map(|i| crate::Coedge::new(
                i, false, crate::line(corners[i].to_vec(), corners[(i + 1) % 4].to_vec()),
            )).collect(),
        });
        belt.faces[1].holes.push(cutout);
        let domain = FaceDomain::new(&belt, 1, 1e-8).unwrap();
        assert_eq!(domain.classify([[0.; 2], [0.78125; 2]], 10_000).unwrap().location, Location::Inside);
        assert_eq!(domain.classify([[0.; 2], [0.875; 2]], 10_000).unwrap().location, Location::Outside);
        assert!(!domain.on_linear_boundary([[1.; 2], [0.25, 0.75]]));
        assert_eq!(domain.classify([[0.375; 2], [0.5; 2]], 10_000).unwrap().location, Location::Outside);
        assert_eq!(domain.classify([[0.625; 2], [0.5; 2]], 10_000).unwrap().location, Location::Inside);
        let mut invalid_hole = belt.clone();
        for coedge in &mut invalid_hole.loops[cutout].coedges {
            for p in &mut coedge.pcurve.control_points {
                p[1] -= 0.25;
            }
        }
        assert!(FaceDomain::new(&invalid_hole, 1, 1e-8).is_err());
        let mut seam_cutout = belt.clone();
        for coedge in &mut seam_cutout.loops[cutout].coedges {
            for p in &mut coedge.pcurve.control_points {
                p[0] -= 1.375;
            }
        }
        let seam_domain = FaceDomain::new(&seam_cutout, 1, 1e-8).unwrap();
        for u in [0.9375, -0.9375] {
            assert_eq!(seam_domain.classify([[u; 2], [0.5; 2]], 10_000).unwrap().location, Location::Outside);
        }
        assert_eq!(seam_domain.classify([[0.75; 2], [0.5; 2]], 10_000).unwrap().location, Location::Inside);
        assert!(!seam_domain.on_linear_boundary([[1.; 2], [0.25, 0.375]]));
        for coedge in &mut seam_cutout.loops[cutout].coedges {
            for p in &mut coedge.pcurve.control_points {
                p[1] -= 0.25;
            }
        }
        assert!(FaceDomain::new(&seam_cutout, 1, 1e-8).is_err());
        let mut off_seam = belt.clone();
        for wire in [outer, hole] {
            for point in &mut off_seam.loops[wire].coedges[0].pcurve.control_points {
                point[0] += 0.5;
            }
        }
        let rotated = FaceDomain::new(&off_seam, 1, 1e-8).unwrap();
        assert_eq!(rotated.classify([[0.625; 2], [0.78125; 2]], 10_000).unwrap().location, Location::Inside);
        assert_eq!(rotated.classify([[0.375; 2], [0.5; 2]], 10_000).unwrap().location, Location::Outside);
        assert!(!rotated.on_linear_boundary([[1.; 2], [0.25, 0.5]]));
        belt.loops[hole].coedges[0].pcurve = belt.loops[hole].coedges[0].pcurve.reverse().unwrap();
        assert!(FaceDomain::new(&belt, 1, 1e-8).is_err());
        let seam = &mut model.loops[outer].coedges[1];
        *seam = seam.clone().with_trim(brep_topology::CoedgeTrim {
            periodic_lift: [[0; 2]; 2], ..trim
        }).unwrap();
        assert!(FaceDomain::new(&model, 1, 1e-8).is_err());
    }

    #[test]
    fn closed_linear_boundary_membership_does_not_expand_the_trim() {
        let model = crate::cuboid([0., 0., 0.], [1., 1., 1.]).unwrap();
        let domain = FaceDomain::new(&model, 0, 1e-7).unwrap();
        assert!(domain.contains_closed_rectangle([[0., 1.], [0., 1.]]));
        assert!(domain.contains_closed_rectangle([[0., 0.], [0.25, 0.75]]));
        assert!(!domain.contains_closed_rectangle([[-1e-15, 1.], [0., 1.]]));
        assert!(domain.on_linear_boundary([[0., 0.], [0.25, 0.75]]));
        assert!(domain.on_linear_boundary([[0.25, 0.75], [1., 1.]]));
        assert!(!domain.on_linear_boundary([[1e-15, 1e-15], [0.25, 0.75]]));
        assert!(!domain.on_linear_boundary([[0., 0.1], [0.25, 0.75]]));
        assert!(!domain.on_linear_boundary([[0., 0.], [-0.1, 0.75]]));
    }
    #[test]
    fn box_faces_use_pcurves_in_loop_order() {
        let model = crate::cuboid([0., 0., 0.], [10., 20., 30.]).unwrap();
        assert!(
            model
                .loops
                .iter()
                .flat_map(|l| &l.coedges)
                .any(|c| c.reversed)
        );
        for i in 0..model.faces.len() {
            let d = FaceDomain::new(&model, i, 1e-7).unwrap();
            assert_eq!(
                d.classify([[0.5; 2]; 2], 1000).unwrap().location,
                Location::Inside
            );
        }
        assert!(FaceDomain::new(&model, 100, 1e-7).is_err());
    }
    #[test]
    fn cap_hole_is_excluded_and_boundary_cells_stay_unresolved() {
        let outer = Curve::from_polyline(vec![
            vec![0., 0.],
            vec![10., 0.],
            vec![10., 10.],
            vec![0., 10.],
            vec![0., 0.],
        ])
        .unwrap();
        let hole = Curve::from_polyline(vec![
            vec![3., 3.],
            vec![3., 7.],
            vec![7., 7.],
            vec![7., 3.],
            vec![3., 3.],
        ])
        .unwrap();
        let model = crate::prism::extrude(&[vec![outer], vec![hole]], 0., 2.).unwrap();
        model.validate().unwrap();
        let d = FaceDomain::new(&model, model.faces.len() - 1, 1e-7).unwrap();
        assert_eq!(
            d.classify([[0.5; 2]; 2], 1000).unwrap().location,
            Location::Outside
        );
        assert_eq!(
            d.classify([[0.1, 0.2], [0.1, 0.2]], 1000).unwrap().location,
            Location::Inside
        );
        assert_eq!(
            d.classify([[0.2, 0.4], [0.4, 0.6]], 1000).unwrap().location,
            Location::Unresolved
        );
        assert!(d.classify([[-0.1, 0.2], [0.1, 0.2]], 1000).is_err());
    }
}

pub struct TrimAudit {
    /// Missing entries remain explicit for unsupported or unvisited faces.
    pub faces: Vec<Option<nurbs_core::trim_region_audit::Report>>,
    pub all_valid: bool,
    pub pairs: usize,
    pub cells: usize,
    pub domain_cells: usize,
}
/// UV-region evidence only; edge lifts, surface embedding and volume validity
/// have separate checks. Every budget is shared by all inspected faces.
pub fn audit_trim_regions(
    model: &Model,
    tolerance_uv: f64,
    max_pairs: usize,
    max_cells: usize,
    max_domain_cells: usize,
) -> Result<TrimAudit> {
    audit_trim_regions_impl(model, tolerance_uv, max_pairs, max_cells, max_domain_cells, false)
}
/// Edge refinement has independently checked both the authored trim
/// correspondence and the complete original use. Its material contour is the
/// stored pcurve, while the partial clock is retained as correspondence data.
pub(crate) fn audit_edge_trim_regions(
    model: &Model,
    tolerance_uv: f64,
    max_pairs: usize,
    max_cells: usize,
    max_domain_cells: usize,
) -> Result<TrimAudit> {
    audit_trim_regions_impl(model, tolerance_uv, max_pairs, max_cells, max_domain_cells, true)
}
fn audit_trim_regions_impl(
    model: &Model,
    tolerance_uv: f64,
    max_pairs: usize,
    max_cells: usize,
    max_domain_cells: usize,
    edge_refinement: bool,
) -> Result<TrimAudit> {
    model.validate_boundary_diagnostic_inputs()?;
    if !(1..=100000).contains(&max_pairs)
        || !(1..=100000).contains(&max_cells)
        || !(1..=1000000).contains(&max_domain_cells)
        || !tolerance_uv.is_finite()
        || tolerance_uv <= 0.
    {
        return Err(Error::new(
            "BREP_INVALID_INPUT",
            "Trim audit requires positive tolerance and bounded work",
        ));
    }
    let mut out = TrimAudit {
        faces: Vec::new(),
        all_valid: !model.faces.is_empty(),
        pairs: 0,
        cells: 0,
        domain_cells: 0,
    };
    for face in &model.faces {
        let (mut loops, periodic) = face_pcurves(model, face, edge_refinement)?;
        if periodic {
            // Prove one-copy coverage before auditing the original outer/hole
            // orientation. Clipping can produce several positive outer loops.
            let (projected, kind) = periodic_wire::project(&face.surface, &loops)?;
            if kind == periodic_wire::Projection::Wrapped {
                loops = projected;
            }
        }
        for wire in &mut loops {
            let singleton = wire.len() == 1;
            let periodic_carrier = singleton
                && wire[0].periodic
                && closed_carrier::literal_multispan(&wire[0]);
            if edge_refinement || periodic_carrier {
                let mut prepared=Vec::new();
                for curve in wire.iter() {
                    if singleton || closed_carrier::literal_multispan(curve) {
                        match closed_carrier::prepare(
                            curve, tolerance_uv, max_cells - out.cells, max_domain_cells - out.domain_cells,
                        ) {
                            Ok((curves, cells, work)) => {
                                out.cells += cells;
                                out.domain_cells += work;
                                prepared.extend(curves);
                            }
                            Err(err) if edge_refinement || err.code != "BREP_CLOSED_UV_REFUSED" => return Err(err),
                            Err(_) => prepared.push(curve.clone()),
                        }
                    } else { prepared.push(curve.clone()); }
                }
                *wire=prepared;
            }
        }
        let result = if out.pairs < max_pairs
            && out.cells < max_cells
            && out.domain_cells < max_domain_cells
            && loops.len() <= 16
            && loops.iter().all(|l| l.len() >= 2)
            && loops.iter().map(Vec::len).sum::<usize>() <= 256
        {
            Some(nurbs_core::trim_region_audit::inspect(
                &loops,
                tolerance_uv,
                max_pairs - out.pairs,
                max_cells - out.cells,
                max_domain_cells - out.domain_cells,
            )?)
        } else {
            None
        };
        if let Some(r) = &result {
            out.pairs += r.pairs;
            out.cells += r.cells;
            out.domain_cells += r.domain_cells;
        }
        out.all_valid &= result.as_ref().is_some_and(|r| r.valid == Some(true));
        out.faces.push(result);
    }
    Ok(out)
}
#[cfg(test)]
mod trim_audit_tests {
    use super::*;
    #[test]
    fn cube_regions_and_global_exhaustion_preserve_face_indices() {
        let m = crate::cuboid([0.; 3], [1.; 3]).unwrap();
        let before = format!("{m:?}");
        let r = audit_trim_regions(&m, 1e-8, 1000, 10000, 100000).unwrap();
        assert!(r.all_valid);
        assert_eq!(r.faces.len(), 6);
        for (p, c, d) in [(1, 10000, 100000), (1000, 1, 100000), (1000, 10000, 1)] {
            let r = audit_trim_regions(&m, 1e-8, p, c, d).unwrap();
            assert!(!r.all_valid);
            assert_eq!(r.faces.len(), 6);
            assert!(r.pairs <= p && r.cells <= c && r.domain_cells <= d);
        }
        assert_eq!(format!("{m:?}"), before);
    }
}
