use crate::{Result, check, core::vec3_ext::norm, curve::Curve};
use math_core::{cross, dot, sub};
type V = [f64; 3];
fn unit(v: V) -> Result<V> {
    crate::core::vec3_ext::unit(v, "Miter direction must be finite and nonzero")
}

/// Open polyline miter sections, retaining rational knots, degrees and weights.
/// Profiles lie in the initial normal plane (relative numerical tolerance 1e-10).
/// Minimum rotations transport the transverse frame; miter_limit bounds sec(angle/2).
/// No global intersection, binary64 exactness or source regularity certificate.
pub fn miter_sections(
    profiles: &[Curve],
    points: &[V],
    normal: V,
    miter_limit: f64,
) -> Result<Vec<Vec<Curve>>> {
    miter_sections_impl(profiles, points, normal, miter_limit, false)
}

/// Cyclic extrusion miter sections. Nonclosing minimum-rotation frames refuse;
/// no seam rotation is hidden by copying unmatched endpoint controls.
pub fn closed_miter_sections(
    profiles: &[Curve],
    points: &[V],
    normal: V,
    miter_limit: f64,
) -> Result<Vec<Vec<Curve>>> {
    miter_sections_impl(profiles, points, normal, miter_limit, true)
}
fn corner_bisector(incoming: V, outgoing: V, limit: f64) -> Result<V> {
    let c = dot(incoming, outgoing).clamp(-1., 1.);
    check(c > -1. + 1e-12, "Miter sweep cannot resolve a reversal")?;
    check(
        (2. / (1. + c)).sqrt() <= limit,
        "Miter corner exceeds the authored limit",
    )?;
    unit(std::array::from_fn(|k| incoming[k] + outgoing[k]))
}
fn miter_sections_impl(
    profiles: &[Curve],
    points: &[V],
    normal: V,
    miter_limit: f64,
    closed: bool,
) -> Result<Vec<Vec<Curve>>> {
    check(
        (1..=64).contains(&profiles.len()),
        "Miter sweep needs 1..64 profiles",
    )?;
    check(
        if closed {
            (3..=16).contains(&points.len())
        } else {
            (2..=17).contains(&points.len())
        },
        "Miter sweep needs 2..17 open or 3..16 cyclic sites",
    )?;
    check(
        points.first() != points.last(),
        "Miter sweep input must not repeat its first site",
    )?;
    check(
        points.iter().flatten().all(|x| x.is_finite()),
        "Miter sites must be finite",
    )?;
    check(
        miter_limit.is_finite() && miter_limit >= 1.,
        "Miter limit must be finite and at least one",
    )?;
    let tangents = (0..points.len() - usize::from(!closed))
        .map(|i| unit(sub(points[(i + 1) % points.len()], points[i])))
        .collect::<Result<Vec<_>>>()?;
    let authored = unit(normal)?;
    let mut n = unit(sub(
        authored,
        tangents[0].map(|x| x * dot(authored, tangents[0])),
    ))?;
    let initial_binormal = cross(tangents[0], n);
    let offsets = profiles
        .iter()
        .map(|profile| {
            profile.validate()?;
            check(
                profile.control_points.iter().all(|p| p.len() == 3),
                "Miter profiles must be 3D",
            )?;
            profile
                .control_points
                .iter()
                .map(|p| {
                    let q = sub([p[0], p[1], p[2]], points[0]);
                    check(
                        dot(q, tangents[0]).abs() <= 1e-10 * norm(q),
                        "Miter profile must lie in the initial normal plane",
                    )?;
                    Ok([dot(q, n), dot(q, initial_binormal)])
                })
                .collect::<Result<Vec<_>>>()
        })
        .collect::<Result<Vec<_>>>()?;
    let initial_normal = n;
    let make_station =
        |position: V, incoming: V, n: V, bisector: Option<V>| -> Result<Vec<Curve>> {
            let binormal = cross(incoming, n);
            profiles
                .iter()
                .zip(&offsets)
                .map(|(profile, offsets)| {
                    let mut section = profile.clone();
                    for (p, offset) in section.control_points.iter_mut().zip(offsets) {
                        let q: V =
                            std::array::from_fn(|k| offset[0] * n[k] + offset[1] * binormal[k]);
                        let shift = bisector.map_or(0., |b| dot(q, b) / dot(incoming, b));
                        *p = (0..3)
                            .map(|k| position[k] + q[k] - shift * incoming[k])
                            .collect();
                    }
                    section.validate()?;
                    Ok(section)
                })
                .collect::<Result<Vec<_>>>()
        };
    let first = if closed {
        make_station(
            points[0],
            tangents[0],
            n,
            Some(corner_bisector(
                *tangents.last().unwrap(),
                tangents[0],
                miter_limit,
            )?),
        )?
    } else {
        profiles.to_vec()
    };
    let mut sections = vec![first];
    for i in 1..points.len() + usize::from(closed) {
        let incoming = tangents[i - 1];
        let bisector = if closed || i < points.len() - 1 {
            Some(corner_bisector(
                incoming,
                tangents[i % points.len()],
                miter_limit,
            )?)
        } else {
            None
        };
        let station = make_station(points[i % points.len()], incoming, n, bisector)?;
        // A positive Bernstein control advance conservatively excludes local
        // reversal of corresponding rational profile sites along this segment.
        for (a, b) in sections.last().unwrap().iter().zip(&station) {
            for (p, q) in a.control_points.iter().zip(&b.control_points) {
                let advance = dot(sub([q[0], q[1], q[2]], [p[0], p[1], p[2]]), incoming);
                check(
                    advance.is_finite() && advance > 0.,
                    "Miter joins consume or reverse a profile segment",
                )?;
            }
        }
        sections.push(station);
        if let Some(b) = bisector {
            let outgoing = tangents[i % points.len()];
            // Equivalent to the minimum rotation from incoming to outgoing.
            let denominator = dot(incoming, b);
            let rotated = sub(n, b.map(|x| x * dot(n, outgoing) / denominator));
            n = unit(sub(rotated, outgoing.map(|x| x * dot(rotated, outgoing))))?;
        }
    }
    if closed {
        check(
            norm(sub(n, initial_normal)) < 1e-10,
            "Closed miter frame holonomy requires an authored twist correction",
        )?;
        // Only matching frames permit exact shared seam controls.
        let last = sections.len() - 1;
        sections[last] = sections[0].clone();
        for (a, b) in sections[last - 1].iter().zip(&sections[last]) {
            for (p, q) in a.control_points.iter().zip(&b.control_points) {
                check(
                    dot(
                        sub([q[0], q[1], q[2]], [p[0], p[1], p[2]]),
                        *tangents.last().unwrap(),
                    ) > 0.,
                    "Miter seam consumes a profile segment",
                )?;
            }
        }
    }
    Ok(sections)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn cyclic_miter_preserves_seam_bisectors_and_segment_extrusion() {
        let points = [[0., 0., 0.], [10., 0., 0.], [10., 10., 0.], [0., 10., 0.]];
        let profile = crate::primitives::line([0., 0.1, 0.2], [0., 0.3, 0.4]).unwrap();
        let sections = closed_miter_sections(&[profile], &points, [0., 0., 1.], 2.).unwrap();
        assert_eq!(sections.len(), 5);
        assert_eq!(sections.first(), sections.last());
        assert_eq!(sections[0][0].control_points[0], vec![0.1, 0.1, 0.2]);
        for i in 0..points.len() {
            let t = unit(sub(points[(i + 1) % points.len()], points[i])).unwrap();
            for (p, q) in sections[i][0]
                .control_points
                .iter()
                .zip(&sections[i + 1][0].control_points)
            {
                let delta = sub([q[0], q[1], q[2]], [p[0], p[1], p[2]]);
                assert!(norm(cross(delta, t)) < 1e-12);
            }
        }
    }
    #[test]
    fn cyclic_miter_refuses_unmatched_frame_holonomy() {
        let profile = crate::primitives::line([0., 0.1, 0.2], [0., 0.3, 0.4]).unwrap();
        let points = [
            [0., 0., 0.],
            [10., 0., 0.],
            [10., 10., 4.],
            [0., 10., 1.],
            [0., 5., -2.],
        ];
        let error = closed_miter_sections(&[profile], &points, [0., 0., 1.], 4.).unwrap_err();
        assert!(error.to_string().contains("holonomy"), "{error}");
    }
    #[test]
    fn right_angle_preserves_rational_profile_and_meets_bisector() {
        let profile = super::super::bezier(
            vec![vec![1., 0., 0.], vec![1.5, 1., 0.], vec![2., 0., 0.]],
            Some(vec![1., 0.7, 2.]),
        )
        .unwrap();
        let result = miter_sections(
            &[profile.clone()],
            &[[0., 0., 0.], [0., 0., 10.], [10., 0., 10.]],
            [1., 0., 0.],
            2.,
        )
        .unwrap();
        assert_eq!(result[0][0], profile);
        for section in &result {
            assert_eq!(section[0].weights, profile.weights);
            assert_eq!(section[0].knots, profile.knots);
        }
        for (p, q) in profile
            .control_points
            .iter()
            .zip(&result[1][0].control_points)
        {
            assert!((q[0] - p[0]).abs() < 1e-12 && (q[2] - (10. - p[0])).abs() < 1e-12);
            assert!((q[0] + q[2] - 10.).abs() < 1e-12);
        }
        assert_eq!(result[2][0].control_points[0], vec![10., 0., 9.]);
    }
    #[test]
    fn spatial_segments_keep_corresponding_transverse_locations() {
        let profile = crate::primitives::line([0.2, 0.1, 0.], [0.4, 0.3, 0.]).unwrap();
        let points = [[0., 0., 0.], [0., 0., 10.], [10., 0., 10.], [10., 10., 15.]];
        let sections = miter_sections(&[profile], &points, [1., 0., 0.], 4.).unwrap();
        for i in 0..points.len() - 1 {
            let t = unit(sub(points[i + 1], points[i])).unwrap();
            for (a, b) in sections[i][0]
                .control_points
                .iter()
                .zip(&sections[i + 1][0].control_points)
            {
                let delta = sub([b[0], b[1], b[2]], [a[0], a[1], a[2]]);
                assert!(norm(cross(delta, t)) < 1e-12);
            }
        }
    }
    #[test]
    fn rejects_unbounded_corners_nonplanar_profiles_and_consumed_segments() {
        let p = crate::primitives::line([1., 0., 0.], [2., 0., 0.]).unwrap();
        let path = [[0., 0., 0.], [0., 0., 10.], [10., 0., 10.]];
        assert!(miter_sections(&[p.clone()], &path, [1., 0., 0.], 1.1).is_err());
        assert!(
            miter_sections(
                &[p.clone()],
                &[[0., 0., 0.], [0., 0., 1.], [1., 0., 1.]],
                [1., 0., 0.],
                2.
            )
            .is_err()
        );
        let q = crate::primitives::line([1., 0., 1.], [2., 0., 1.]).unwrap();
        assert!(miter_sections(&[q], &path, [1., 0., 0.], 2.).is_err());
        assert!(
            miter_sections(
                &[p],
                &[[0., 0., 0.], [0., 0., 1.], [0., 0., 0.]],
                [1., 0., 0.],
                4.
            )
            .is_err()
        );
    }
}
