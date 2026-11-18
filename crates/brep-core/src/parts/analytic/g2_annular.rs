//! Quintic C2 corner transitions for an annular cylindrical solid. The native
//! construction is distinct from seam, embedding and volume qualification.
use crate::{Error, Model, Result};
use nurbs_core::{curve::Curve, interval_eval::Interval};
pub struct Construction {
    pub model: Model,
    pub transition_size: f64,
    pub transition_sizes: [f64; 4],
    pub radial_wall_thickness_lower: f64,
    pub angular_radius_bounds: [f64; 2],
}
impl Construction {
    /// Integer dual-map proposals only. The native chart checker independently
    /// proves every inequality on the actual saved surface under its own budget.
    pub fn projection_hints(&self) -> Vec<Option<[[i8; 3]; 2]>> {
        let signs = [
            [1, 1],
            [0, 1],
            [-1, 1],
            [-1, 0],
            [-1, -1],
            [0, -1],
            [1, -1],
            [1, 0],
        ];
        self.model
            .faces
            .iter()
            .enumerate()
            .map(|(index, face)| {
                let j = (index / 4) % 8;
                let q = index % 4;
                let a = super::QUADRANTS[q];
                let b = super::QUADRANTS[(q + 1) % 4];
                let [r, z] = signs[j];
                let radius = face
                    .surface
                    .control_points
                    .iter()
                    .flatten()
                    .flat_map(|p| p[..2].iter())
                    .map(|x| x.abs())
                    .fold(0_f64, f64::max);
                let width = if j % 2 == 0 {
                    self.transition_sizes[j / 2]
                } else if r == 0 {
                    radius
                } else {
                    self.radial_wall_thickness_lower
                };
                let scale = (radius / width).ceil().clamp(1., 64.) as i8;
                Some([
                    [(b[0] - a[0]) as i8 * scale, (b[1] - a[1]) as i8 * scale, 0],
                    [(a[0] + b[0]) as i8 * r, (a[1] + b[1]) as i8 * r, z],
                ])
            })
            .collect()
    }
}
/// Sealed proof owner: monotone polynomial segments in pairwise separated
/// control hull boxes, with adjacent boxes meeting at only their shared endpoint.
/// Positive unit weights preserve each hull and strict derivative projection.
pub(super) struct Meridian {
    wire: Vec<Curve>,
}
impl Meridian {
    pub(super) fn wire(&self) -> &[Curve] {
        &self.wire
    }
    fn new(wire: Vec<Curve>, directions: &[[f64; 2]]) -> Result<Self> {
        let refused = || {
            Error::new(
                "BREP_G2_MERIDIAN_UNPROVEN",
                "Saved meridian monotonicity, hull separation or endpoint identity is unproved",
            )
        };
        if wire.len() != 8 || directions.len() != 8 {
            return Err(refused());
        }
        let mut boxes = Vec::new();
        for (i, c) in wire.iter().enumerate() {
            c.validate()?;
            if c.degree != if i % 2 == 0 { 5 } else { 1 }
                || c.control_points.len() != c.degree + 1
                || c.weights.iter().any(|&w| w != 1.)
                || c.control_points.iter().any(|p| p.len() != 2 || p[0] <= 0.)
                || c.knots != [vec![0.; c.degree + 1], vec![1.; c.degree + 1]].concat()
            {
                return Err(refused());
            }
            if c.control_points.last() != Some(&wire[(i + 1) % 8].control_points[0]) {
                return Err(refused());
            }
            for pair in c.control_points.windows(2) {
                let mut strict = false;
                for k in 0..2 {
                    let (a, b, d) = (pair[0][k], pair[1][k], directions[i][k]);
                    if d == 0. {
                        if a != b {
                            return Err(refused());
                        }
                    } else if d > 0. {
                        if b < a {
                            return Err(refused());
                        }
                        strict |= b > a;
                    } else {
                        if b > a {
                            return Err(refused());
                        }
                        strict |= b < a;
                    }
                }
                if !strict {
                    return Err(refused());
                }
            }
            boxes.push(std::array::from_fn::<_, 2, _>(|k| {
                [
                    c.control_points
                        .iter()
                        .map(|p| p[k])
                        .fold(f64::INFINITY, f64::min),
                    c.control_points
                        .iter()
                        .map(|p| p[k])
                        .fold(f64::NEG_INFINITY, f64::max),
                ]
            }));
        }
        for a in 0..8 {
            for b in a + 1..8 {
                let adjacent = b == a + 1 || (a == 0 && b == 7);
                if adjacent {
                    let shared = if b == a + 1 {
                        wire[a].control_points.last().unwrap()
                    } else {
                        &wire[0].control_points[0]
                    };
                    if !(0..2).all(|k| {
                        boxes[a][k][0].max(boxes[b][k][0]) == shared[k]
                            && boxes[a][k][1].min(boxes[b][k][1]) == shared[k]
                    }) {
                        return Err(refused());
                    }
                } else if !(0..2)
                    .any(|k| boxes[a][k][1] < boxes[b][k][0] || boxes[b][k][1] < boxes[a][k][0])
                {
                    return Err(refused());
                }
            }
        }
        Ok(Self { wire })
    }
}
fn curve(points: Vec<Vec<f64>>) -> Curve {
    let degree = points.len() - 1;
    Curve {
        degree,
        knots: std::iter::repeat_n(0., degree + 1)
            .chain(std::iter::repeat_n(1., degree + 1))
            .collect(),
        weights: vec![1.; points.len()],
        control_points: points,
        periodic: false,
    }
}
/// Choose endpoint handle lengths on the local binary64 lattice. This keeps
/// consecutive saved poles in exact arithmetic progression even for decimal
/// extents; the authored corner extent and endpoints are unchanged.
fn endpoint_step(origin: f64, requested: f64) -> f64 {
    let magnitude = origin.abs().max(requested.abs() * 2.);
    let quantum = magnitude.next_up() - magnitude;
    (requested / quantum).round() * quantum
}
/// The extent is measured along both incident meridian lines. Keep a positive
/// flat annulus and cylindrical wall; never silently clamp an oversized request.
/// Second endpoint jets vanish, so adjoining straight meridians have C2/G2
/// under their independent speeds in real arithmetic. Saved jets still require
/// independent rounding-inclusive inspection before any G2 claim.
pub fn cylinder(
    outer: f64,
    inner: f64,
    height: f64,
    extent: f64,
    tolerance: f64,
) -> Result<Construction> {
    cylinder_with_transitions(outer, inner, height, [extent; 4], tolerance)
}
/// Independent outer-bottom, outer-top, inner-top and inner-bottom extents.
/// These are G2 blend extents, not a claim of constant circular radius.
pub fn cylinder_with_transitions(
    outer: f64,
    inner: f64,
    height: f64,
    extents: [f64; 4],
    tolerance: f64,
) -> Result<Construction> {
    if ![outer, inner, height, tolerance]
        .iter()
        .chain(extents.iter())
        .all(|x| x.is_finite())
        || inner <= 0.
        || outer <= inner
        || height <= 0.
        || tolerance <= 0.
        || extents.iter().any(|&e| e <= 0.)
        || extents[0] + extents[3] >= outer - inner
        || extents[1] + extents[2] >= outer - inner
        || extents[0] + extents[1] >= height
        || extents[2] + extents[3] >= height
    {
        return Err(Error::new(
            "BREP_G2_ANNULAR_LIMITS",
            "Adjacent positive transition extents must fit their common wall or cap",
        ));
    }
    let extent = extents.iter().copied().fold(0_f64, f64::max);
    // Stored quarter-circle weights are binary64, not exact sqrt(1/2).
    // For [1,w,1], |A(t)|² = 1 + (4w²-2)t²(1-t)²/W(t)².
    // Enclose this factor before claiming a physical radial separation.
    let w = Interval::point(super::WEIGHT);
    let w2 = w.mul(w)?;
    let k = w2.mul(Interval::point(4.))?.sub(Interval::point(2.))?;
    let q2 = Interval::point(1.).add(k.mul(Interval::new(0., 1. / 16.)?.div(w2)?)?)?;
    let q = Interval::new(q2.lo.sqrt().next_down(), q2.hi.sqrt().next_up())?;
    let angular_radius_bounds = [q.lo, q.hi];
    let outer_min = Interval::point(outer)
        .sub(Interval::point(extents[0].max(extents[1])))?
        .mul(q)?
        .lo;
    let inner_max = Interval::point(inner)
        .add(Interval::point(extents[2].max(extents[3])))?
        .mul(q)?
        .hi;
    let radial_wall_thickness_lower = Interval::point(outer_min)
        .sub(Interval::point(inner_max))?
        .lo;
    if radial_wall_thickness_lower <= 0. {
        return Err(Error::new(
            "BREP_G2_ANNULAR_LIMITS",
            "Radial wall thickness is not separated from zero",
        ));
    }
    let corners = [[outer, 0.], [outer, height], [inner, height], [inner, 0.]];
    let directions = [[0., 1.], [-1., 0.], [0., -1.], [1., 0.]];
    let mut wire = Vec::new();
    let mut monotone = Vec::new();
    for i in 0..4 {
        let extent = extents[i];
        let incoming = directions[(i + 3) % 4];
        let outgoing = directions[i];
        let c = corners[i];
        let start = std::array::from_fn::<_, 2, _>(|k| c[k] - extent * incoming[k]);
        let end = std::array::from_fn::<_, 2, _>(|k| c[k] + extent * outgoing[k]);
        // A dyadic quarter of the extent avoids unnecessary fifth-denominator
        // rounding in stored poles. The resulting endpoint speed is 5r/4.
        let step = extent * 0.25;
        let controls = [
            start,
            std::array::from_fn(|k| start[k] + endpoint_step(start[k], step) * incoming[k]),
            std::array::from_fn(|k| start[k] + 2. * endpoint_step(start[k], step) * incoming[k]),
            std::array::from_fn(|k| end[k] - 2. * endpoint_step(end[k], step) * outgoing[k]),
            std::array::from_fn(|k| end[k] - endpoint_step(end[k], step) * outgoing[k]),
            end,
        ];
        wire.push(curve(controls.into_iter().map(|p| p.to_vec()).collect()));
        monotone.push(std::array::from_fn(|k| incoming[k] + outgoing[k]));
        let next = corners[(i + 1) % 4];
        let stop = std::array::from_fn::<_, 2, _>(|k| next[k] - extents[(i + 1) % 4] * outgoing[k]);
        wire.push(curve(vec![end.to_vec(), stop.to_vec()]));
        monotone.push(outgoing);
    }
    let meridian = Meridian::new(wire, &monotone)?;
    let model = super::revolve_g2_annular_meridian(&meridian, tolerance)?;
    Ok(Construction {
        model,
        transition_size: extent,
        transition_sizes: extents,
        radial_wall_thickness_lower,
        angular_radius_bounds,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn independent_cap_wall_extents_keep_g2_and_positive_wall_separation() {
        let built=cylinder_with_transitions(10.,7.,20.,[0.1,0.25,0.5,0.75],1e-8).unwrap();
        assert_eq!(built.transition_sizes,[0.1,0.25,0.5,0.75]);
        let jets=crate::sweep_smoothness::inspect_profile(&built.model,&[],2_000_000).unwrap();
        assert!(jets.profile.g2_certified&&jets.station.g2_certified);
        assert!(built.radial_wall_thickness_lower>1.99);
        for extents in [[2.5,0.25,0.5,0.75],[0.1,2.5,0.5,0.1],[0.1,0.25,-0.5,0.75]] {
            assert!(cylinder_with_transitions(10.,7.,20.,extents,1e-8).is_err());
        }
    }
    #[test]
    fn decimal_transition_keeps_exact_saved_endpoint_jets() {
        let built=cylinder(10.,7.,20.,0.1,1e-8).unwrap();
        let jets=crate::sweep_smoothness::inspect_profile(&built.model,&[],2_000_000).unwrap();
        for (e,p) in jets.station.edge_ids.iter().zip(&jets.station.g2.seams) {if !p.certified {eprintln!("station {e}: {p:?}");}}
        assert!(jets.profile.g2_certified&&jets.station.g2_certified,"profile={} station={} work={}",jets.profile.g2_certified,jets.station.g2_certified,jets.total_exact_work);
    }
    #[test]
    fn rounded_annular_cylinder_has_saved_g2_at_every_wall_and_cap_join() {
        let built=cylinder(8.,4.,16.,0.5,1e-8).unwrap();
        built.model.validate().unwrap();
        assert_eq!(built.model.faces.len(),32);
        assert_eq!(built.model.shells.len(),1);assert!(built.model.shells[0].closed);
        assert_eq!(built.model.bodies.len(),1);
        assert!(built.radial_wall_thickness_lower>2.99&&built.radial_wall_thickness_lower<=3.);
        let before=built.model.clone();
        let jets=crate::sweep_smoothness::inspect_profile(&built.model,&[],2_000_000).unwrap();
        eprintln!("rounded G2 profile={} station={} work={} profile_seams={} station_seams={}",jets.profile.g2_certified,jets.station.g2_certified,jets.total_exact_work,jets.profile.edge_ids.len(),jets.station.edge_ids.len());
        for (axis,r) in [("profile",&jets.profile),("station",&jets.station)] {for (edge,p) in r.edge_ids.iter().zip(&r.g2.seams) {if !p.certified{eprintln!("{axis} edge={edge} {p:?}");}}}
        assert!(jets.profile.extraction_complete&&jets.station.extraction_complete);
        assert_eq!(jets.profile.edge_ids.len()+jets.station.edge_ids.len(),built.model.edges.len());
        assert!(jets.profile.g2_certified&&jets.station.g2_certified,"Saved transitions and all angular seams must have independent exact G2 proofs");
        assert!(jets.total_exact_work<=2_000_000);
        assert_eq!(built.model,before);
    }
    #[test]
    fn oversized_or_collapsed_g2_transitions_are_refused() {
        for extent in [0.,-0.5,2.,4.,f64::NAN,f64::INFINITY] {assert!(cylinder(8.,4.,16.,extent,1e-8).is_err());}
        assert!(cylinder(8.,4.,1.,0.5,1e-8).is_err());
        assert!(cylinder(4.,4.,16.,0.5,1e-8).is_err());
    }
}
