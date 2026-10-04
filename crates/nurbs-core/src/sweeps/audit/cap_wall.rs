//! Sufficient coordinate-plane exclusion of cap/wall interior contacts.
//! A boundary-restricted contact still requires curve ownership and cap trim
//! correspondence; this certificate alone never grants an allowed intersection.
use crate::{Result, check, surface::Surface};
#[derive(Clone, Copy, Debug)]
pub enum Boundary {
    UMin,
    UMax,
    VMin,
    VMax,
}
#[derive(Clone, Debug)]
pub struct Report {
    pub all_wall_interiors_excluded: bool,
    pub plane_axis: Option<usize>,
    pub inspected_walls: usize,
    pub separated_walls: Vec<usize>,
    pub boundary_restricted_walls: Vec<usize>,
    pub unresolved_walls: Vec<usize>,
    pub reason: Option<&'static str>,
}
fn endpoint(s: &Surface, boundary: Boundary) -> Option<(usize, usize)> {
    let (axis, end) = match boundary {
        Boundary::UMin => (0, 0),
        Boundary::UMax => (0, 1),
        Boundary::VMin => (1, 0),
        Boundary::VMax => (1, 1),
    };
    let (degree, knots, count, periodic) = if axis == 0 {
        (s.degree_u, &s.knots_u, s.control_points.len(), s.periodic_u)
    } else {
        (
            s.degree_v,
            &s.knots_v,
            s.control_points[0].len(),
            s.periodic_v,
        )
    };
    if periodic {
        return None;
    }
    let range = if end == 0 {
        &knots[..=degree]
    } else {
        &knots[count..]
    };
    let value = knots[if end == 0 { degree } else { count }];
    if range.len() != degree + 1 || range.iter().any(|k| *k != value) {
        return None;
    }
    Some((axis, if end == 0 { 0 } else { count - 1 }))
}
/// Whole boundary image, not injectivity or contact ownership. A continuous
/// positive rational curve stays in its control hull and attains both endpoint
/// values; the intermediate value theorem covers every point of the side.
pub fn covers_boundary(s: &Surface, p: &crate::curve::Curve, boundary: Boundary) -> Result<bool> {
    s.validate()?;
    p.validate()?;
    check(
        p.control_points[0].len() == 2,
        "Boundary coverage requires a 2D pcurve",
    )?;
    let Some((axis, _)) = endpoint(s, boundary) else {
        return Ok(false);
    };
    if p.periodic {
        return Ok(false);
    }
    let count = p.control_points.len();
    let start = p.knots[p.degree];
    let end = p.knots[count];
    if p.knots[..=p.degree].iter().any(|k| *k != start)
        || p.knots[count..].iter().any(|k| *k != end)
    {
        return Ok(false);
    }
    let domains = crate::sweep_support::audit::surface_domains(s);
    let value = domains[axis][usize::from(matches!(boundary, Boundary::UMax | Boundary::VMax))];
    let other = 1 - axis;
    let [lo, hi] = domains[other];
    if p.control_points
        .iter()
        .any(|v| v[axis] != value || v[other] < lo || v[other] > hi)
    {
        return Ok(false);
    }
    let a = p.control_points[0][other];
    let b = p.control_points[count - 1][other];
    Ok((a == lo && b == hi) || (a == hi && b == lo))
}
pub fn inspect(
    cap: &Surface,
    walls: &[Surface],
    boundaries: &[Option<Boundary>],
    max_walls: usize,
) -> Result<Report> {
    cap.validate()?;
    check(
        (1..=1024).contains(&walls.len()) && boundaries.len() == walls.len() && max_walls <= 1024,
        "Invalid cap/wall size or budget",
    )?;
    for s in std::iter::once(cap).chain(walls) {
        s.validate()?;
        check(
            s.control_points.iter().flatten().all(|p| p.len() == 3),
            "Cap/wall audit requires 3D charts",
        )?;
    }
    let plane_axis = (0..3).find(|axis| {
        let value = cap.control_points[0][0][*axis];
        cap.control_points
            .iter()
            .flatten()
            .all(|p| p[*axis] == value)
    });
    let mut out = Report {
        all_wall_interiors_excluded: false,
        plane_axis,
        inspected_walls: 0,
        separated_walls: Vec::new(),
        boundary_restricted_walls: Vec::new(),
        unresolved_walls: Vec::new(),
        reason: None,
    };
    let Some(axis) = plane_axis else {
        out.unresolved_walls = (0..walls.len()).collect();
        out.reason = Some("cap-coordinate-plane-unproved");
        return Ok(out);
    };
    let plane = cap.control_points[0][0][axis];
    for (index, wall) in walls.iter().enumerate() {
        if out.inspected_walls == max_walls {
            out.unresolved_walls.extend(index..walls.len());
            out.reason = Some("cap-wall-budget-exhausted");
            break;
        }
        out.inspected_walls += 1;
        let one_side = |points: Vec<&Vec<f64>>| {
            points.iter().all(|p| p[axis] > plane) || points.iter().all(|p| p[axis] < plane)
        };
        if one_side(wall.control_points.iter().flatten().collect()) {
            out.separated_walls.push(index);
            continue;
        }
        if let Some((normal, endpoint)) =
            boundaries[index].and_then(|boundary| endpoint(wall, boundary))
        {
            let mut boundary = Vec::new();
            let mut remaining = Vec::new();
            for (u, row) in wall.control_points.iter().enumerate() {
                for (v, point) in row.iter().enumerate() {
                    if [u, v][normal] == endpoint {
                        boundary.push(point);
                    } else {
                        remaining.push(point);
                    }
                }
            }
            if boundary.iter().all(|p| p[axis] == plane)
                && !remaining.is_empty()
                && one_side(remaining)
            {
                out.boundary_restricted_walls.push(index);
                continue;
            }
        }
        out.unresolved_walls.push(index);
    }
    out.all_wall_interiors_excluded = out.unresolved_walls.is_empty();
    if !out.all_wall_interiors_excluded && out.reason.is_none() {
        out.reason = Some("cap-wall-interior-exclusion-unproved");
    }
    Ok(out)
}
#[cfg(test)]
mod tests {
    use super::*;
    fn plane(z0: f64, z1: f64) -> Surface {
        Surface {
            degree_u: 1,
            degree_v: 1,
            knots_u: vec![0., 0., 1., 1.],
            knots_v: vec![0., 0., 1., 1.],
            control_points: vec![
                vec![vec![0., 0., z0], vec![0., 1., z1]],
                vec![vec![1., 0., z0], vec![1., 1., z1]],
            ],
            weights: vec![vec![1., 2.], vec![3., 4.]],
            periodic_u: false,
            periodic_v: false,
        }
    }
    #[test]
    fn whole_side_coverage_uses_continuity_and_exact_endpoints() {
        use crate::curve::Curve;
        let s = plane(0., 5.);
        let mut p = Curve {
            degree: 2,
            knots: vec![0., 0., 0., 1., 1., 1.],
            control_points: vec![vec![0., 0.], vec![0.9, 0.], vec![1., 0.]],
            weights: vec![1., 3., 2.],
            periodic: false,
        };
        assert!(covers_boundary(&s, &p, Boundary::VMin).unwrap());
        assert!(covers_boundary(&s, &p.reverse().unwrap(), Boundary::VMin).unwrap());
        p.control_points[2][0] = 0.75;
        assert!(!covers_boundary(&s, &p, Boundary::VMin).unwrap());
        p.control_points[2][0] = 1.;
        p.control_points[1][0] = 1_f64.next_up();
        assert!(!covers_boundary(&s, &p, Boundary::VMin).unwrap());
        p.control_points[1] = vec![0.9, f64::EPSILON];
        assert!(!covers_boundary(&s, &p, Boundary::VMin).unwrap());
        let discontinuous = Curve {
            degree: 1,
            knots: vec![0., 0., 0.5, 0.5, 1., 1.],
            control_points: vec![vec![0., 0.], vec![0.25, 0.], vec![0.75, 0.], vec![1., 0.]],
            weights: vec![1.; 4],
            periodic: false,
        };
        assert!(covers_boundary(&s, &discontinuous, Boundary::VMin).is_err());
        let mut natural = s.clone();
        natural.knots_u = vec![-2., -2., 3., 3.];
        let q = Curve {
            degree: 1,
            knots: vec![0., 0., 1., 1.],
            control_points: vec![vec![-2., 0.], vec![3., 0.]],
            weights: vec![1., 2.],
            periodic: false,
        };
        assert!(covers_boundary(&natural, &q, Boundary::VMin).unwrap());
        let request = value_codec::json!({"op":"sweep_boundary_coverage_audit","surface":natural,"uv":q,"boundary":"vMin"});
        let report = crate::transport::dispatch(request).unwrap();
        assert_eq!(report["wholeBoundaryCovered"], true);
        assert_eq!(report["injectivityCertified"], false);
    }
    #[test]
    fn whole_wall_exclusion_distinguishes_boundaries_and_unresolved_crossings() {
        let cap = plane(0., 0.);
        let walls = [plane(0., 1.), plane(1., 2.)];
        let r = inspect(&cap, &walls, &[Some(Boundary::VMin), None], 2).unwrap();
        assert!(r.all_wall_interiors_excluded);
        #[cfg(feature = "transport")]
        {
            let request = value_codec::json!({"op":"sweep_cap_wall_audit","cap":cap,
                "walls":walls,"boundaries":[Some("vMin"),None::<&str>],"maxWalls":2});
            let response = crate::transport::dispatch(request.clone()).unwrap();
            assert_eq!(
                response["allWallInteriorsExcluded"],
                value_codec::json!(true)
            );
            assert_eq!(
                response["boundaryOwnershipCertified"],
                value_codec::json!(false)
            );
            let mut exhausted = request.clone();
            exhausted["maxWalls"] = value_codec::json!(0);
            let response = crate::transport::dispatch(exhausted).unwrap();
            assert_eq!(response["unresolvedWalls"], value_codec::json!([0, 1]));
            let mut invalid = request;
            invalid["boundaries"] = value_codec::json!([Some("invalid"), None::<&str>]);
            assert!(crate::transport::dispatch(invalid).is_err());
        }

        assert_eq!(r.boundary_restricted_walls, vec![0]);
        assert_eq!(r.separated_walls, vec![1]);
        assert!(
            !inspect(&cap, &walls, &[None, None], 2)
                .unwrap()
                .all_wall_interiors_excluded
        );
        let crossing = plane(-1., 1.);
        assert!(
            !inspect(&cap, &[crossing], &[Some(Boundary::VMin)], 1)
                .unwrap()
                .all_wall_interiors_excluded
        );
        let mut folded = walls[0].clone();
        folded.control_points[1][1][2] = -1.;
        assert!(
            !inspect(&cap, &[folded], &[Some(Boundary::VMin)], 1)
                .unwrap()
                .all_wall_interiors_excluded
        );
        let r = inspect(&cap, &walls, &[Some(Boundary::VMin), None], 1).unwrap();
        assert!(!r.all_wall_interiors_excluded);
        assert_eq!(r.unresolved_walls, vec![1]);
        assert_eq!(r.reason, Some("cap-wall-budget-exhausted"));
        let r = inspect(&cap, &walls, &[Some(Boundary::VMin), None], 0).unwrap();
        assert_eq!(r.inspected_walls, 0);
        assert_eq!(r.unresolved_walls, vec![0, 1]);
        let mut nonplanar = cap.clone();
        nonplanar.control_points[1][1][2] = 0.1;
        let r = inspect(&nonplanar, &walls, &[Some(Boundary::VMin), None], 2).unwrap();
        assert!(!r.all_wall_interiors_excluded && r.plane_axis.is_none());
        assert_eq!(r.reason, Some("cap-coordinate-plane-unproved"));
    }
}
