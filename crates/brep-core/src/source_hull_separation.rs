//! Exact strict separation of complete original rational control hulls.
use crate::source_contour_proposal::SourceRegion;
use cad_predicates::Sign;
use nurbs_core::{Error, Result};
pub struct Certificate {
    regions: [SourceRegion; 2],
    plane: [[f64; 3]; 3],
    sides: [Sign; 2],
    material_hulls: [Option<crate::source_planar_material_hull::Certificate>; 2],
}
impl Certificate {
    pub fn material_hulls(&self) -> &[Option<crate::source_planar_material_hull::Certificate>; 2] {
        &self.material_hulls
    }
    pub fn regions(&self) -> &[SourceRegion; 2] {
        &self.regions
    }
    pub fn plane(&self) -> [[f64; 3]; 3] {
        self.plane
    }
    pub fn sides(&self) -> [Sign; 2] {
        self.sides
    }
}
pub struct Report {
    pub certificate: Option<Certificate>,
    pub exact_work: u64,
    pub reason: &'static str,
}
/// Centroids and coordinate gaps propose planes only. Fresh strict signs of every original
/// control prove the separation, and positive weights enclose every trimmed
/// region in its original control hull. Unsuccessful guesses are unresolved.
pub fn certify(regions: [&SourceRegion; 2], max_work: u64) -> Result<Report> {
    certify_points(regions, original_points(regions), [None, None], max_work)
}
fn original_points(regions: [&SourceRegion; 2]) -> [Vec<[f64; 3]>; 2] {
    regions.map(|r| {
        r.loops()[0][0]
            .surface()
            .control_points
            .iter()
            .flatten()
            .map(|p| p.as_slice().try_into().unwrap())
            .collect()
    })
}
/// Fresh native planar material enclosures replace only proven planar hulls.
/// Curved and unsupported trimmed charts retain their full original controls.
pub fn certify_shell(
    shell: &crate::source_shell_incidence::Shell,
    faces: [usize; 2],
    max_work: u64,
) -> Result<Report> {
    if !(1..=100_000_000).contains(&max_work) || faces[0] == faces[1] {
        return Err(Error::new(
            "BREP_SOURCE_CONTACT",
            "Choose distinct faces and bounded hull work",
        ));
    }
    let all = shell
        .regions()
        .ok_or_else(|| Error::new("BREP_SOURCE_CONTACT", "Original regions required"))?;
    if faces.iter().any(|f| *f >= all.len()) {
        return Err(Error::new("BREP_SOURCE_CONTACT", "Invalid hull face"));
    }
    let regions = faces.map(|f| &all[f]);
    let mut initial = certify(regions, max_work)?;
    if initial.certificate.is_some() {
        return Ok(initial);
    }
    let mut hulls = [None, None];
    let mut used = initial.exact_work;
    for slot in 0..2 {
        if used == max_work {
            return Ok(Report {
                certificate: None,
                exact_work: used,
                reason: "source-hull-work-limit",
            });
        }
        let r = crate::source_planar_material_hull::certify(shell, faces[slot], max_work - used)?;
        used += r.exact_work;
        hulls[slot] = r.certificate;
    }
    if used == max_work {
        return Ok(Report {
            certificate: None,
            exact_work: used,
            reason: "source-hull-work-limit",
        });
    }
    if hulls.iter().all(Option::is_none) {
        initial.exact_work = used;
        return Ok(initial);
    }
    let original = original_points(regions);
    let points = std::array::from_fn(|slot| {
        hulls[slot]
            .as_ref()
            .map(|h| h.points().to_vec())
            .unwrap_or_else(|| original[slot].clone())
    });
    let mut r = certify_points(regions, points, hulls, max_work - used)?;
    r.exact_work += used;
    Ok(r)
}
fn certify_points(
    regions: [&SourceRegion; 2],
    points: [Vec<[f64; 3]>; 2],
    material_hulls: [Option<crate::source_planar_material_hull::Certificate>; 2],
    max_work: u64,
) -> Result<Report> {
    if !(1..=100_000_000).contains(&max_work) {
        return Err(Error::new("BREP_SOURCE_CONTACT", "Bound source hull work"));
    }
    let mut out = Report {
        certificate: None,
        exact_work: 0,
        reason: "source-hull-plane-unproven",
    };
    let centers = points.each_ref().map(|points| {
        let count = points.len() as f64;
        std::array::from_fn::<_, 3, _>(|k| points.iter().map(|p| p[k] / count).sum::<f64>())
    });
    let mut proposals = Vec::new();
    let centroid_plane = (|| {
        let mut normal = std::array::from_fn::<_, 3, _>(|k| centers[0][k] - centers[1][k]);
        let scale = normal.iter().map(|v| v.abs()).fold(0_f64, f64::max);
        if scale == 0. || !scale.is_finite() {
            return None;
        }
        for x in &mut normal {
            *x /= scale;
        }
        let axis = (0..3)
            .min_by(|&a, &b| normal[a].abs().total_cmp(&normal[b].abs()))
            .unwrap();
        let mut basis = [0.; 3];
        basis[axis] = 1.;
        let cross = |a: [f64; 3], b: [f64; 3]| {
            [
                a[1] * b[2] - a[2] * b[1],
                a[2] * b[0] - a[0] * b[2],
                a[0] * b[1] - a[1] * b[0],
            ]
        };
        let x = cross(normal, basis);
        let y = cross(normal, x);
        let point = std::array::from_fn::<_, 3, _>(|k| centers[0][k] * 0.5 + centers[1][k] * 0.5);
        let plane = [
            point,
            std::array::from_fn(|k| point[k] + x[k]),
            std::array::from_fn(|k| point[k] + y[k]),
        ];
        if plane.iter().flatten().any(|v| !v.is_finite()) {
            return None;
        }
        Some(plane)
    })();
    if let Some(plane) = centroid_plane {
        proposals.push(plane);
    }
    // Coordinate gaps propose additional planes. Midpoints are never proof:
    // every original/enclosed control must freshly pass exact strict signs.
    for axis in 0..3 {
        let bounds = points.each_ref().map(|ps| {
            ps.iter().fold([f64::INFINITY, f64::NEG_INFINITY], |b, p| {
                [b[0].min(p[axis]), b[1].max(p[axis])]
            })
        });
        let gap = if bounds[0][1] < bounds[1][0] {
            Some([bounds[0][1], bounds[1][0]])
        } else if bounds[1][1] < bounds[0][0] {
            Some([bounds[1][1], bounds[0][0]])
        } else {
            None
        };
        if let Some([lo, hi]) = gap {
            let mid = lo * 0.5 + hi * 0.5;
            if !mid.is_finite() {
                continue;
            }
            let mut p = [0.; 3];
            p[axis] = mid;
            let mut q = p;
            q[(axis + 1) % 3] = 1.;
            let mut r = p;
            r[(axis + 2) % 3] = 1.;
            proposals.push([p, q, r]);
        }
    }
    for plane in proposals {
        if !crate::source_allowed_contact::independent(
            plane[0],
            plane[1],
            plane[2],
            &mut out.exact_work,
            max_work,
        )? {
            if out.exact_work == max_work {
                return Ok(out);
            }
            continue;
        }
        let mut sides = [None; 2];
        let mut valid = true;
        for (slot, points) in points.iter().enumerate() {
            for p in points {
                let Some(sign) = crate::source_allowed_contact::orient(
                    &[plane[0], plane[1], plane[2], *p],
                    None,
                    &mut out.exact_work,
                    max_work,
                )?
                else {
                    out.reason = "source-hull-work-limit";
                    return Ok(out);
                };
                if sign == Sign::Zero || sides[slot].is_some_and(|s| s != sign) {
                    out.reason = "source-hull-strict-side-unproven";
                    valid = false;
                    break;
                }
                sides[slot] = Some(sign);
            }
            if !valid {
                break;
            }
        }
        if valid && sides[0] != sides[1] {
            out.certificate = Some(Certificate {
                regions: [regions[0].clone(), regions[1].clone()],
                plane,
                sides: sides.map(Option::unwrap),
                material_hulls,
            });
            out.reason = "source-hull-disjoint-qualified";
            return Ok(out);
        }
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    fn shell(radii: [f64; 2], sweep: f64) -> crate::source_shell_incidence::Shell {
        let spans = crate::linear_canal::construct(
            [[10., -7., 5.], [13., -3., 17.]],
            radii,
            [1., 0., 0.],
            sweep,
        )
        .unwrap();
        crate::linear_canal::to_capped_source_shell(
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
        .unwrap()
    }
    #[test]
    fn rounded_midpoint_cannot_admit_a_zero_contact_sign() {
        // Isolate the exact proposal admission with original binary inputs.
        // Synthetic point hulls exercise rounding, not shell qualification.
        let shell = shell([1., 1.], std::f64::consts::TAU);
        let regions = shell.regions().unwrap();
        let refs = [&regions[0], &regions[1]];
        let one = 1_f64;
        let adjacent = f64::from_bits(one.to_bits() + 1);
        let separated = f64::from_bits(one.to_bits() + 2);
        let test = |x, budget| {
            certify_points(
                refs,
                [vec![[one, 0., 0.]], vec![[x, 0., 0.]]],
                [None, None],
                budget,
            )
            .unwrap()
        };
        assert!(test(adjacent, 1000000).certificate.is_none());
        assert!(test(one, 1000000).certificate.is_none());
        assert!(test(separated, 1000000).certificate.is_some());
        assert!(test(separated, 1).certificate.is_none());
    }
    #[test]
    fn strict_original_hulls_separate_opposite_rotated_shaft_faces() {
        for radii in [[0.5, 1.25], [1.25, 0.5], [1., 1.]] {
            for sweep in [std::f64::consts::TAU, -std::f64::consts::TAU] {
                let shell = shell(radii, sweep);
                let regions = shell.regions().unwrap();
                let shafts = regions
                    .iter()
                    .enumerate()
                    .filter_map(|(i, r)| (r.loops()[0][0].surface().degree_u == 1).then_some(i))
                    .collect::<Vec<_>>();
                for faces in [[shafts[0], shafts[2]], [shafts[2], shafts[0]]] {
                    let r = certify([&regions[faces[0]], &regions[faces[1]]], 1000000).unwrap();
                    let c = r.certificate.expect(r.reason);
                    assert_ne!(c.sides()[0], c.sides()[1]);
                    assert!(r.exact_work > 0 && r.exact_work <= 1000000);
                    assert!(
                        certify([&regions[faces[0]], &regions[faces[1]]], 1)
                            .unwrap()
                            .certificate
                            .is_none()
                    );
                }
                assert!(
                    certify([&regions[0], &regions[0]], 1000000)
                        .unwrap()
                        .certificate
                        .is_none()
                );
            }
        }
    }
    #[test]
    fn a_shared_cone_pole_cannot_be_reported_as_disjoint() {
        let shell = shell([0., 1.], std::f64::consts::TAU);
        let regions = shell.regions().unwrap();
        let shafts = regions
            .iter()
            .enumerate()
            .filter_map(|(i, r)| (r.loops()[0][0].surface().degree_u == 1).then_some(i))
            .collect::<Vec<_>>();
        assert!(
            certify([&regions[shafts[0]], &regions[shafts[2]]], 1000000)
                .unwrap()
                .certificate
                .is_none()
        );
        assert!(certify([&regions[0], &regions[1]], 0).is_err());
    }
}
