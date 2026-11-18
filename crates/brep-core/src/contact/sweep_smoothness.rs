//! Native ownership and sufficient exact jets of retained wall seams.
//! Caps, trim/world-edge identity and global shell validity remain separate.
pub mod complete;
pub mod geometric;
pub mod tangent;
use crate::{Error, Face, MAX_COEDGES, MAX_ENTITIES, MAX_FACES, Model, Result};
use nurbs_core::{
    continuity::{self, ProjectiveSeam, ProjectiveSeamCollectionReport},
    curve::Curve,
    surface::Surface,
};

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Axis {
    Profile,
    Station,
}
pub struct AxisReport {
    pub max_work: u64,
    pub exact_work: u64,
    pub extraction_complete: bool,
    pub unclassified_faces: Vec<usize>,
    pub unpaired_edges: Vec<usize>,
    pub cap_edges: Vec<usize>,
    pub edge_ids: Vec<usize>,
    pub g2: ProjectiveSeamCollectionReport,
    pub g1: Option<ProjectiveSeamCollectionReport>,
    pub g1_certified: bool,
    pub g2_certified: bool,
}
pub struct ProfileReport {
    pub profile: AxisReport,
    pub station: AxisReport,
    pub total_exact_work: u64,
    pub cap_continuity: &'static str,
    pub station_continuity: &'static str,
}
fn require(value: bool, message: &str) -> Result<()> {
    if value {
        Ok(())
    } else {
        Err(Error::new("BREP_SWEEP_SMOOTHNESS_INVALID", message))
    }
}
pub(crate) fn boundary(s: &Surface, p: &Curve, axis: Axis) -> Option<&'static str> {
    if p.degree != 1
        || p.control_points.len() != 2
        || p.weights.len() != 2
        || p.weights[0] != p.weights[1]
        || p.weights[0] <= 0.
        || p.control_points.iter().any(|v| v.len() != 2)
        || s.validate().is_err()
        || p.validate().is_err()
    {
        return None;
    }
    boundary_from_endpoints(s, &p.control_points[0], &p.control_points[1], axis)
}

fn boundary_from_endpoints(s: &Surface, a: &[f64], b: &[f64], axis: Axis) -> Option<&'static str> {
    let coordinate = usize::from(axis == Axis::Station);
    let other = 1 - coordinate;
    let domains = [
        [s.knots_u[s.degree_u], s.knots_u[s.control_points.len()]],
        [s.knots_v[s.degree_v], s.knots_v[s.control_points[0].len()]],
    ];
    if a[coordinate] != b[coordinate]
        || !((a[other] == domains[other][0] && b[other] == domains[other][1])
            || (a[other] == domains[other][1] && b[other] == domains[other][0]))
    {
        return None;
    }
    let end = if a[coordinate] == domains[coordinate][0] {
        0
    } else if a[coordinate] == domains[coordinate][1] {
        1
    } else {
        return None;
    };
    // This classifies an exact full parameter-side use, including a
    // nonclamped or periodic side. It grants no world-edge correspondence,
    // regularity or closure credit: each audit proves those independently.
    Some(match (axis, end) {
        (Axis::Profile, 0) => "uMin",
        (Axis::Profile, _) => "uMax",
        (Axis::Station, 0) => "vMin",
        (Axis::Station, _) => "vMax",
    })
}
/// Full parameter side of a coedge (profile first, then station) and whether
/// its use runs along increasing free parameter.
pub(crate) fn coedge_side(
    s: &Surface,
    pcurve: &Curve,
    reversed: bool,
) -> Option<(&'static str, bool)> {
    let label = boundary(s, pcurve, Axis::Profile).or_else(|| boundary(s, pcurve, Axis::Station))?;
    let free = usize::from(label.starts_with('u'));
    let [first, last] = pcurve.control_points.as_slice() else {
        return None;
    };
    Some((label, (last.get(free) > first.get(free)) != reversed))
}
/// Upper bound on the exact work one retained smoothness audit may spend.
const MAX_SMOOTHNESS_WORK: u64 = 2_000_000;
fn invalid(message: &str) -> Error {
    Error::new("BREP_SWEEP_SMOOTHNESS_INVALID", message)
}
fn face_wires(face: &Face) -> impl Iterator<Item = usize> + '_ {
    std::iter::once(face.outer).chain(face.holes.iter().copied())
}
fn require_limits(model: &Model, max_work: u64) -> Result<()> {
    require(
        max_work <= MAX_SMOOTHNESS_WORK
            && model.faces.len() <= MAX_FACES
            && model.edges.len() <= MAX_ENTITIES
            && model.loops.len() <= MAX_ENTITIES,
        "Invalid retained smoothness limits",
    )?;
    require(
        model.loops.iter().map(|w| w.coedges.len()).sum::<usize>() <= MAX_COEDGES,
        "Too many retained smoothness coedges",
    )
}
/// Cap faces and the edges their loops own.
struct CapScope {
    caps: Vec<bool>,
    cap_owned: Vec<bool>,
}
fn cap_scope(model: &Model, cap_faces: &[usize]) -> Result<CapScope> {
    let mut caps = vec![false; model.faces.len()];
    let mut cap_owned = vec![false; model.edges.len()];
    for &face in cap_faces {
        require(
            face < caps.len() && !caps[face],
            "Invalid station smoothness cap scope",
        )?;
        caps[face] = true;
        for wire in face_wires(&model.faces[face]) {
            let wire = model.loops.get(wire).ok_or_else(|| invalid("Invalid cap loop"))?;
            for c in &wire.coedges {
                require(c.edge < cap_owned.len(), "Invalid cap edge")?;
                cap_owned[c.edge] = true;
            }
        }
    }
    Ok(CapScope { caps, cap_owned })
}
/// Exact full parameter-side uses of every wall edge, in first-seen order.
struct WallUses {
    uses: Vec<Vec<(usize, &'static str)>>,
    edge_order: Vec<usize>,
    unclassified_faces: Vec<usize>,
}
fn wall_uses(model: &Model, caps: &[bool], axis: Axis) -> Result<WallUses> {
    let mut out = WallUses {
        uses: vec![Vec::new(); model.edges.len()],
        edge_order: Vec::with_capacity(model.edges.len()),
        unclassified_faces: Vec::new(),
    };
    for (index, face) in model.faces.iter().enumerate() {
        if caps[index] {
            continue;
        }
        let mut found = [false; 2];
        for wire in face_wires(face) {
            let wire = model.loops.get(wire).ok_or_else(|| invalid("Invalid wall loop"))?;
            for c in &wire.coedges {
                require(c.edge < out.uses.len(), "Invalid wall edge")?;
                if let Some(label) = boundary(&face.surface, &c.pcurve, axis) {
                    found[usize::from(label.ends_with("Max"))] = true;
                    if out.uses[c.edge].is_empty() {
                        out.uses[c.edge].reserve_exact(2);
                        out.edge_order.push(c.edge);
                    }
                    out.uses[c.edge].push((index, label));
                }
            }
        }
        if !found.into_iter().all(|v| v) {
            out.unclassified_faces.push(index);
        }
    }
    Ok(out)
}
/// Paired wall seams plus the edges that stayed caps or unpaired.
struct SeamPairs {
    seams: Vec<ProjectiveSeam>,
    edge_ids: Vec<usize>,
    cap_edges: Vec<usize>,
    unpaired_edges: Vec<usize>,
}
fn pair_seams(
    model: &Model,
    walls: &WallUses,
    cap_owned: &[bool],
    axis: Axis,
) -> Result<SeamPairs> {
    let mut out = SeamPairs {
        seams: Vec::with_capacity(walls.edge_order.len()),
        edge_ids: Vec::with_capacity(walls.edge_order.len()),
        cap_edges: Vec::new(),
        unpaired_edges: Vec::new(),
    };
    for &edge in &walls.edge_order {
        let list = &walls.uses[edge];
        let station_cap = axis == Axis::Station && cap_owned[edge];
        if station_cap && list.len() == 1 {
            out.cap_edges.push(edge);
            continue;
        }
        let [(a, ab), (b, bb)] = match list.as_slice() {
            [first, second] if !station_cap => [*first, *second],
            _ => {
                out.unpaired_edges.push(edge);
                continue;
            }
        };
        let normal_scale = if axis == Axis::Profile {
            1.
        } else {
            continuity::propose_boundary_normal_scale(
                &model.faces[a].surface,
                &model.faces[b].surface,
                ab,
                bb,
            )?
        };
        out.seams.push(ProjectiveSeam {
            patches: [a, b],
            boundaries: [ab.into(), bb.into()],
            order: 2,
            normal_scale,
        });
        out.edge_ids.push(edge);
    }
    Ok(out)
}
/// One collection proof; an incomplete extraction never certifies.
fn inspect_seams(
    patches: &[Surface],
    seams: &[ProjectiveSeam],
    max_work: u64,
    extraction_complete: bool,
) -> Result<ProjectiveSeamCollectionReport> {
    let mut report =
        continuity::inspect_projective_seam_collection_until_refused(patches, seams, max_work)?;
    if !extraction_complete {
        report.certified = false;
        report.certified_order = None;
    }
    Ok(report)
}
pub fn inspect_axis(
    model: &Model,
    cap_faces: &[usize],
    axis: Axis,
    max_work: u64,
) -> Result<AxisReport> {
    require_limits(model, max_work)?;
    let CapScope { caps, cap_owned } = cap_scope(model, cap_faces)?;
    let walls = wall_uses(model, &caps, axis)?;
    let SeamPairs {
        mut seams,
        edge_ids,
        cap_edges,
        unpaired_edges,
    } = pair_seams(model, &walls, &cap_owned, axis)?;
    let unclassified_faces = walls.unclassified_faces;
    let patches = model
        .faces
        .iter()
        .map(|f| f.surface.clone())
        .collect::<Vec<_>>();
    let extraction_complete = unclassified_faces.is_empty() && unpaired_edges.is_empty();
    let g2 = inspect_seams(&patches, &seams, max_work, extraction_complete)?;
    let mut exact_work = g2.exact_work;
    let g1 = if g2.certified {
        None
    } else {
        for seam in &mut seams {
            seam.order = 1;
        }
        let proof = inspect_seams(&patches, &seams, max_work - exact_work, extraction_complete)?;
        exact_work += proof.exact_work;
        Some(proof)
    };
    let g2_certified = g2.certified;
    let g1_certified =
        extraction_complete && (g2_certified || g1.as_ref().is_some_and(|r| r.certified));
    Ok(AxisReport {
        max_work,
        exact_work,
        extraction_complete,
        unclassified_faces,
        unpaired_edges,
        cap_edges,
        edge_ids,
        g2,
        g1,
        g1_certified,
        g2_certified,
    })
}
pub fn inspect_profile(model: &Model, cap_faces: &[usize], max_work: u64) -> Result<ProfileReport> {
    let profile = inspect_axis(model, cap_faces, Axis::Profile, max_work)?;
    let station = inspect_axis(
        model,
        cap_faces,
        Axis::Station,
        max_work - profile.exact_work,
    )?;
    let total_exact_work = profile.exact_work + station.exact_work;
    let station_continuity = if station.g2_certified {
        "G2"
    } else if station.g1_certified {
        "G1"
    } else {
        "C0"
    };
    Ok(ProfileReport {
        profile,
        station,
        total_exact_work,
        cap_continuity: if cap_faces.is_empty() { "absent" } else { "C0" },
        station_continuity,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn unequal_profile_strip_speeds_prove_only_the_actual_smooth_seam() {
        let corners = [[0., 0.], [1., 0.], [3., 0.], [3., 1.], [0., 1.], [0., 0.]];
        let sections = [0., 1.]
            .into_iter()
            .map(|z| {
                vec![
                    corners
                        .windows(2)
                        .map(|p| {
                            nurbs_core::primitives::line(
                                [p[0][0], p[0][1], z],
                                [p[1][0], p[1][1], z],
                            )
                            .unwrap()
                        })
                        .collect::<Vec<_>>(),
                ]
            })
            .collect::<Vec<_>>();
        let model = crate::rational_section_loft(&sections).unwrap();
        let caps = [model.faces.len() - 2, model.faces.len() - 1];
        let before = model.clone();
        let unit = continuity::inspect_projective_seam_collection_until_refused(
            &model
                .faces
                .iter()
                .map(|f| f.surface.clone())
                .collect::<Vec<_>>(),
            &[ProjectiveSeam {
                patches: [0, 1],
                boundaries: ["uMax".into(), "uMin".into()],
                order: 2,
                normal_scale: 1.,
            }],
            2000000,
        )
        .unwrap();
        assert!(
            unit.certified,
            "Exact binary-ratio fallback must cover unequal profile speeds even with the unit proposal"
        );
        let proof = inspect_axis(&model, &caps, Axis::Profile, 2000000).unwrap();
        assert!(proof.extraction_complete);
        assert!(
            !proof.g2_certified && !proof.g1_certified,
            "Sharp corners remain C0"
        );
        assert!(
            proof.g2.seams[0].certified,
            "The collinear seam has unequal represented speeds: {:?}",
            proof.g2.seams
        );
        assert!(proof.g2.seams.iter().skip(1).any(|s| !s.certified));
        assert_eq!(
            model.faces[0].surface.control_points,
            before.faces[0].surface.control_points
        );
        let zero = inspect_axis(&model, &caps, Axis::Profile, 0).unwrap();
        assert!(!zero.g2_certified && zero.g2.seams.iter().all(|s| !s.certified));
        let short = inspect_axis(&model, &caps, Axis::Profile, proof.g2.seams[0].work - 1).unwrap();
        assert!(!short.g2.seams[0].certified);
        let mut kink = model.clone();
        for p in &mut kink.faces[0].surface.control_points[0] {
            p[1] = f64::EPSILON;
        }
        let damaged = inspect_axis(&kink, &caps, Axis::Profile, 2000000).unwrap();
        assert!(
            !damaged.g2.seams[0].certified,
            "A nonzero transverse kink must remain unproved"
        );
    }
    #[test]
    fn unequal_profile_closing_seam_uses_owned_surfaces_and_exact_ratio() {
        let corners = [[1., 0.], [3., 0.], [3., 1.], [0., 1.], [0., 0.], [1., 0.]];
        let sections = [0., 1.]
            .into_iter()
            .map(|z| {
                vec![
                    corners
                        .windows(2)
                        .map(|p| {
                            nurbs_core::primitives::line(
                                [p[0][0], p[0][1], z],
                                [p[1][0], p[1][1], z],
                            )
                            .unwrap()
                        })
                        .collect::<Vec<_>>(),
                ]
            })
            .collect::<Vec<_>>();
        let model = crate::rational_section_loft(&sections).unwrap();
        let seams = [ProjectiveSeam {
            patches: [0, 4],
            boundaries: ["uMin".into(), "uMax".into()],
            order: 2,
            normal_scale: 1.,
        }];
        assert!(
            model.loops[model.faces[0].outer]
                .coedges
                .iter()
                .any(
                    |a| boundary(&model.faces[0].surface, &a.pcurve, Axis::Profile) == Some("uMin")
                        && model.loops[model.faces[4].outer]
                            .coedges
                            .iter()
                            .any(|b| a.edge == b.edge
                                && boundary(&model.faces[4].surface, &b.pcurve, Axis::Profile)
                                    == Some("uMax"))
                ),
            "The tested closing seam must be a shared owned world edge"
        );
        let audit = |model: &Model, work| {
            continuity::inspect_projective_seam_collection_until_refused(
                &model
                    .faces
                    .iter()
                    .map(|f| f.surface.clone())
                    .collect::<Vec<_>>(),
                &seams,
                work,
            )
            .unwrap()
        };
        let proof = audit(&model, 2000000);
        assert!(
            proof.certified && proof.seams[0].regularity_certified,
            "{:?}",
            proof.seams
        );
        assert!(!audit(&model, proof.exact_work - 1).certified);
        assert!(!audit(&model, 0).certified);
        let mut kink = model.clone();
        for p in &mut kink.faces[0].surface.control_points[1] {
            p[1] = f64::EPSILON;
        }
        assert!(!audit(&kink, 2000000).certified);
    }
    #[test]
    fn sharp_rational_profiles_preserve_nonuniform_station_g2_work() {
        let corners = [[-1., -1.], [1., -1.], [1., 1.], [-1., 1.], [-1., -1.]];
        let mut z = 0.;
        let sections = (0..49)
            .map(|station| {
                if station > 0 {
                    z += if station % 2 == 0 { 2. } else { 1. };
                }
                vec![
                    corners
                        .windows(2)
                        .map(|pair| Curve {
                            degree: 2,
                            knots: vec![0., 0., 0., 1., 1., 1.],
                            control_points: vec![
                                vec![pair[0][0], pair[0][1], z],
                                vec![
                                    (pair[0][0] + pair[1][0]) * 0.5,
                                    (pair[0][1] + pair[1][1]) * 0.5,
                                    z,
                                ],
                                vec![pair[1][0], pair[1][1], z],
                            ],
                            weights: vec![1., 0.5, 1.],
                            periodic: false,
                        })
                        .collect::<Vec<_>>(),
                ]
            })
            .collect::<Vec<_>>();
        let model = crate::rational_section_loft(&sections).unwrap();
        let caps = [model.faces.len() - 2, model.faces.len() - 1];
        let isolated = inspect_axis(&model, &caps, Axis::Station, 2000000).unwrap();
        assert!(
            isolated.extraction_complete && isolated.g2_certified,
            "station work={}",
            isolated.exact_work
        );
        let combined = inspect_profile(&model, &caps, 2000000).unwrap();
        assert!(!combined.profile.g1_certified && !combined.profile.g2_certified);
        assert!(
            combined.station.g2_certified,
            "combined work={}",
            combined.total_exact_work
        );
        assert_eq!(combined.station_continuity, "G2");
        assert!(combined.total_exact_work <= 2000000);
        assert_eq!(
            combined.profile.g2.seams.len(),
            combined.profile.edge_ids.len()
        );
        assert_eq!(
            combined.profile.g2.unresolved_seams.len(),
            combined.profile.edge_ids.len()
        );
        assert!(
            combined
                .profile
                .g2
                .seams
                .iter()
                .skip(1)
                .all(|s| !s.certified && !s.exact_identity && s.work == 0)
        );
        let short = inspect_profile(&model, &caps, combined.total_exact_work - 1).unwrap();
        assert!(!short.station.g2_certified);
        assert!(short.total_exact_work <= combined.total_exact_work - 1);
        eprintln!(
            "sharp-profile independent station G2 work={} combined profile={} total={}",
            isolated.exact_work, combined.profile.exact_work, combined.total_exact_work
        );
    }
    #[test]
    fn profile_g1_curvature_jump_keeps_independent_station_g2() {
        let directions = [[1., 0.], [0., 1.], [-1., 0.], [0., -1.], [1., 0.]];
        let sections = [0., 1., 3.]
            .into_iter()
            .map(|z| {
                vec![
                    directions
                        .windows(2)
                        .enumerate()
                        .map(|(i, pair)| Curve {
                            degree: 2,
                            knots: vec![0., 0., 0., 1., 1., 1.],
                            control_points: vec![
                                vec![pair[0][0], pair[0][1], z],
                                vec![pair[0][0] + pair[1][0], pair[0][1] + pair[1][1], z],
                                vec![pair[1][0], pair[1][1], z],
                            ],
                            weights: vec![1., if i == 0 { 1. } else { 0.5 }, 1.],
                            periodic: false,
                        })
                        .collect::<Vec<_>>(),
                ]
            })
            .collect::<Vec<_>>();
        let model = crate::rational_section_loft(&sections).unwrap();
        let caps = [model.faces.len() - 2, model.faces.len() - 1];
        let proof = inspect_profile(&model, &caps, 2000000).unwrap();
        assert!(
            proof.profile.extraction_complete
                && proof.profile.g1_certified
                && !proof.profile.g2_certified
        );
        assert_eq!(proof.profile.g1.as_ref().unwrap().certified_order, Some(1));
        assert!(proof.station.g2_certified);
        assert_eq!(proof.station_continuity, "G2");
        assert_eq!(proof.cap_continuity, "C0");
        assert!(proof.total_exact_work <= 2000000);
    }
    #[test]
    fn actual_circle_walls_own_profile_station_and_cap_sets_in_one_budget() {
        for stations in [&[0., 5., 10.][..], &[0., 3., 10., 21., 34.][..]] {
            let sections = stations
                .iter()
                .map(|&z| {
                    vec![vec![
                        nurbs_core::primitives::circle([0., 0., z], [0., 0., 1.], 0.5).unwrap(),
                    ]]
                })
                .collect::<Vec<_>>();
            let model = crate::rational_section_loft(&sections).unwrap();
            let caps = [model.faces.len() - 2, model.faces.len() - 1];
            let r = inspect_profile(&model, &caps, 2000000).unwrap();
            assert!(
                r.profile.extraction_complete && r.profile.g2_certified && r.profile.g1_certified
            );
            assert!(
                r.station.extraction_complete && r.station.g2_certified && r.station.g1_certified
            );
            assert_eq!(r.station.edge_ids.len(), 4 * (stations.len() - 2));
            assert_eq!(r.station.cap_edges.len(), 8);
            assert_eq!(
                r.total_exact_work,
                r.profile.exact_work + r.station.exact_work
            );
            assert!(r.total_exact_work <= 2000000);
            let short = inspect_profile(&model, &caps, r.profile.exact_work - 1).unwrap();
            assert!(!short.profile.g2_certified);
            let mut damaged = model;
            let wire = damaged.faces[0].outer;
            let c = damaged.loops[wire]
                .coedges
                .iter_mut()
                .find(|c| c.pcurve.control_points[0][0] == c.pcurve.control_points[1][0])
                .unwrap();
            c.pcurve.control_points[1][1] = 0.5;
            let r = inspect_profile(&damaged, &caps, 2000000).unwrap();
            assert!(
                !r.profile.extraction_complete
                    && !r.profile.g1_certified
                    && !r.profile.g2_certified
            );
            assert!(
                r.profile.unclassified_faces.contains(&0) && !r.profile.unpaired_edges.is_empty()
            );
        }
    }
    #[test]
    fn malformed_boundary_and_invalid_topology_never_produce_positive_ownership() {
        let sections = [0., 4., 12.]
            .iter()
            .map(|&z| {
                vec![vec![
                    nurbs_core::primitives::circle([0., 0., z], [0., 0., 1.], 0.5).unwrap(),
                ]]
            })
            .collect::<Vec<_>>();
        let model = crate::rational_section_loft(&sections).unwrap();
        let caps = [model.faces.len() - 2, model.faces.len() - 1];
        let mut damaged = model.clone();
        let wire = damaged.faces[0].outer;
        for c in &mut damaged.loops[wire].coedges {
            if c.pcurve.control_points[0][0] == c.pcurve.control_points[1][0] {
                c.pcurve.knots = vec![0., 0., 1.];
            }
        }
        let r = inspect_profile(&damaged, &caps, 2000000).unwrap();
        assert!(
            !r.profile.extraction_complete && !r.profile.g1_certified && !r.profile.g2_certified
        );
        let mut invalid = model.clone();
        invalid.faces[0].outer = invalid.loops.len();
        assert!(inspect_profile(&invalid, &caps, 2000000).is_err());
        let mut invalid = model.clone();
        invalid.loops[wire].coedges[0].edge = invalid.edges.len();
        assert!(inspect_profile(&invalid, &caps, 2000000).is_err());
        assert!(inspect_profile(&model, &[caps[0], caps[0]], 2000000).is_err());
        assert!(inspect_profile(&model, &caps, 2000001).is_err());
        let zero = inspect_profile(&model, &caps, 0).unwrap();
        assert!(zero.profile.extraction_complete && zero.station.extraction_complete);
        assert!(!zero.profile.g1_certified && !zero.station.g1_certified);
        assert_eq!(zero.total_exact_work, 0);
    }

    #[test]
    fn station_g1_is_distinct_from_g2_and_spends_only_shared_remainder() {
        let sections = [0., 6., 12.]
            .iter()
            .map(|&z| {
                vec![vec![
                    nurbs_core::primitives::circle([0., 0., z], [0., 0., 1.], 0.5).unwrap(),
                ]]
            })
            .collect::<Vec<_>>();
        let mut model = crate::rational_section_loft(&sections).unwrap();
        let caps = [model.faces.len() - 2, model.faces.len() - 1];
        for face in &mut model.faces[..caps[0]] {
            let s = &mut face.surface;
            for row in &mut s.control_points {
                let a = row[0].clone();
                let b = row[1].clone();
                let shift = if a[2] == 6. { 0.25 } else { 0. };
                *row = vec![
                    a.clone(),
                    vec![a[0], a[1], a[2] + 2.],
                    vec![b[0] + shift, b[1], b[2] - 2.],
                    b,
                ];
            }
            for row in &mut s.weights {
                *row = vec![row[0], row[0], row[1], row[1]];
            }
            s.degree_v = 3;
            s.knots_v = vec![0., 0., 0., 0., 1., 1., 1., 1.];
        }
        let r = inspect_axis(&model, &caps, Axis::Station, 2000000).unwrap();
        assert!(r.extraction_complete && r.g1_certified && !r.g2_certified);
        assert_eq!(r.g1.as_ref().unwrap().certified_order, Some(1));
        assert_eq!(
            r.exact_work,
            r.g2.exact_work + r.g1.as_ref().unwrap().exact_work
        );
        let cutoff = inspect_axis(&model, &caps, Axis::Station, r.g2.exact_work).unwrap();
        assert!(cutoff.extraction_complete && !cutoff.g1_certified && !cutoff.g2_certified);
        assert_eq!(cutoff.g1.as_ref().unwrap().exact_work, 0);
        let wire = model.faces[0].outer;
        let c = model.loops[wire]
            .coedges
            .iter_mut()
            .find(|c| c.pcurve.control_points[0][1] == c.pcurve.control_points[1][1])
            .unwrap();
        c.pcurve.periodic = true;
        let r = inspect_axis(&model, &caps, Axis::Station, 2000000).unwrap();
        assert!(!r.extraction_complete && !r.g1_certified && !r.g2_certified);
    }

    #[test]
    fn closed_hollow_miter_audits_every_station_including_closure_as_c0() {
        let outer = nurbs_core::primitives::circle([0., 0., 0.], [1., 0., 0.], 0.5).unwrap();
        let mut inner = nurbs_core::primitives::circle([0., 0., 0.], [1., 0., 0.], 0.2).unwrap();
        inner.control_points.reverse();
        inner.weights.reverse();
        let rows = nurbs_core::construct::curves::paths::closed_miter_sections(
            &[outer, inner],
            &[[0., 0., 0.], [10., 0., 0.], [10., 10., 0.], [0., 10., 0.]],
            [0., 0., 1.],
            2.,
        )
        .unwrap();
        let sections = rows
            .into_iter()
            .map(|r| r.into_iter().map(|c| vec![c]).collect())
            .collect::<Vec<_>>();
        let model = crate::periodic_section_loft(&sections).unwrap();
        let r = inspect_profile(&model, &[], 2000000).unwrap();
        assert!(r.profile.extraction_complete && r.profile.g2_certified);
        assert!(
            r.station.extraction_complete && !r.station.g1_certified && !r.station.g2_certified
        );
        assert_eq!(r.profile.edge_ids.len(), 32);
        assert_eq!(r.station.edge_ids.len(), 32);
        assert!(r.station.cap_edges.is_empty());
        assert_eq!(r.cap_continuity, "absent");
        assert_eq!(r.station_continuity, "C0");
        assert!(r.total_exact_work <= 2000000);
        assert_eq!(model.shells.len(), 2);
    }
}
