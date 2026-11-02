//! Common exact affine station packet for walls AND complete filled caps.
use super::*;
#[derive(Clone, Debug)]
pub struct AffineBoundaryReport {
    pub coherent_controls: bool,
    pub wall_displacement_upper: Option<f64>,
    pub cap_correction_upper: [f64; 2],
    pub work: u64,
    pub reason: &'static str,
}
pub(super) struct Candidate {
    pub nominal_surfaces: [nurbs_core::surface::Surface; 2],
    pub source_surfaces: [nurbs_core::surface::Surface; 2],
    pub original: Vec<Vec<Curve>>,
    pub origin: [f64; 3],
    pub pieces: Vec<nurbs_core::affine_volume::RationalPiece>,
    pub caps: [LoftCap; 2],
    pub sides: Vec<Vec<nurbs_core::surface::Surface>>,
    pub start: Vec<Vec<Curve>>,
    pub end: Vec<Vec<Curve>>,
}
fn charge(report: &mut AffineBoundaryReport, max: u64) -> bool {
    if report.work == max {
        report.reason = "affine-boundary-work-limit";
        false
    } else {
        report.work += 1;
        true
    }
}
pub(super) fn prepare(
    original: &[Vec<Curve>],
    old: &[Vec<nurbs_core::surface::Surface>],
    caps: [&LoftCap; 2],
    l: crate::AffineStationLimits,
) -> Result<(Option<Candidate>, AffineBoundaryReport)> {
    let mut report = AffineBoundaryReport {
        coherent_controls: false,
        wall_displacement_upper: None,
        cap_correction_upper: [0.; 2],
        work: 0,
        reason: "affine-boundary-coefficient-family-unproved",
    };
    let Some(template) = old.first().and_then(|r| r.first()) else {
        return Ok((None, report));
    };
    let n = template.control_points[0].len();
    if n < 2
        || template.degree_v != 1
        || template.knots_v.len() != n + 2
        || original.len() != old.len()
        || original.iter().zip(old).any(|(a, b)| a.len() != b.len())
    {
        return Ok((None, report));
    }
    let mut sections = vec![original.to_vec(); n];
    for (ring, wire) in old.iter().enumerate() {
        for (span, s) in wire.iter().enumerate() {
            let c = &original[ring][span];
            if s.degree_u != c.degree
                || s.degree_v != 1
                || s.knots_v != template.knots_v
                || s.control_points.len() != c.control_points.len()
                || s.control_points.iter().any(|r| r.len() != n)
                || s.weights
                    .iter()
                    .zip(&c.weights)
                    .any(|(r, w)| r.len() != n || r.iter().any(|x| x != w))
            {
                return Ok((None, report));
            }
            for (i, row) in s.control_points.iter().enumerate() {
                for (j, p) in row.iter().enumerate() {
                    if !charge(&mut report, l.work) {
                        return Ok((None, report));
                    }
                    sections[j][ring][span].control_points[i] = p.clone();
                }
            }
        }
    }
    let proposed = crate::analytic::affine_station_controls(
        original,
        &sections,
        crate::AffineStationLimits {
            work: l.work - report.work,
            ..l
        },
    )?;
    report.work += proposed.work;
    report.reason = proposed.reason;
    let (Some(rows), Some(poses), Some(origin), Some(delta)) = (
        proposed.sections,
        proposed.poses,
        proposed.source_origin,
        proposed.displacement_upper,
    ) else {
        return Ok((None, report));
    };
    let mut retained = [caps[0].clone(), caps[1].clone()];
    let mut source_surfaces = [caps[0].surface.clone(), caps[1].surface.clone()];
    for endpoint in 0..2 {
        let pose = if endpoint == 0 {
            poses[0][0]
        } else {
            poses.last().unwrap()[5]
        };
        let mut failed = false;
        for p in retained[endpoint]
            .surface
            .control_points
            .iter_mut()
            .flatten()
        {
            if !charge(&mut report, l.work) {
                return Ok((None, report));
            }
            if let Some(image) = crate::exact_affine_source::image(pose, p, origin) {
                *p = image.to_vec();
            } else {
                failed = true;
                break;
            }
        }
        if failed {
            // A pure-translation endpoint admits a common-amplitude proposal.
            // Arbitrary affine endpoint correction remains explicitly unproved.
            if pose[..3] != [[1., 0., 0.], [0., 1., 0.], [0., 0., 1.]] {
                report.reason = "affine-filled-cap-exact-product-unproved";
                return Ok((None, report));
            }
            let mut translation = [0.; 3];
            for k in 0..3 {
                let Some(t) = crate::exact_affine_source::exact_add(pose[3][k], -origin[k]) else {
                    report.reason = "affine-filled-cap-translation-unrepresentable";
                    return Ok((None, report));
                };
                translation[k] = t;
            }
            let correction = crate::rational_cap_translation::propose(
                caps[endpoint],
                translation,
                origin,
                l.tolerance,
                l.work - report.work,
            )?;
            report.work += correction.work;
            let Some(cap) = correction.cap else {
                report.reason = correction.reason;
                return Ok((None, report));
            };
            report.cap_correction_upper[endpoint] = correction.displacement_upper.unwrap();
            source_surfaces[endpoint] = cap.surface.clone();
            for p in source_surfaces[endpoint]
                .control_points
                .iter_mut()
                .flatten()
            {
                for k in 0..3 {
                    if !charge(&mut report, l.work) {
                        return Ok((None, report));
                    }
                    let Some(value) = crate::exact_affine_source::exact_add(p[k], -translation[k])
                    else {
                        report.reason = "affine-corrected-cap-reference-unrepresentable";
                        return Ok((None, report));
                    };
                    p[k] = value;
                }
            }
            retained[endpoint] = cap;
        }
    }
    let mut sides = Vec::with_capacity(original.len());
    for (ring, wire) in original.iter().enumerate() {
        let mut patches = Vec::with_capacity(wire.len());
        for (span, c) in wire.iter().enumerate() {
            let mut s = old[ring][span].clone();
            for i in 0..c.control_points.len() {
                for j in 0..n {
                    if !charge(&mut report, l.work) {
                        return Ok((None, report));
                    }
                    s.control_points[i][j] = rows[j][ring][span].control_points[i].clone();
                }
            }
            patches.push(s);
        }
        sides.push(patches);
    }
    report.coherent_controls = true;
    report.wall_displacement_upper = Some(delta);
    report.reason = "exact-affine-filled-boundary-controls";
    Ok((
        Some(Candidate {
            nominal_surfaces: [caps[0].surface.clone(), caps[1].surface.clone()],
            source_surfaces,
            original: original.to_vec(),
            origin,
            pieces: poses
                .iter()
                .map(|p| nurbs_core::affine_volume::RationalPiece {
                    controls: vec![p[0], p[5]],
                    weights: vec![1.; 2],
                })
                .collect(),
            caps: retained,
            sides,
            start: rows[0].clone(),
            end: rows[n - 1].clone(),
        }),
        report,
    ))
}
