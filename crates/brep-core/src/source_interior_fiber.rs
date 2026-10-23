//! Exact interior plane preimage of an unchanged tensor-product source chart.
//! Region support and shared-boundary ownership are separate admission gates.
use nurbs_core::{curve::Curve, surface::Surface, Error, Result};
pub struct Certificate {
    surface: Surface,
    plane: [[f64; 3]; 3],
    axis: usize,
    level: f64,
}
impl Certificate {
    pub fn surface(&self) -> &Surface {
        &self.surface
    }
    pub fn plane(&self) -> [[f64; 3]; 3] {
        self.plane
    }
    pub fn coordinate(&self) -> (usize, f64) {
        (self.axis, self.level)
    }
}
pub struct Report {
    pub certificate: Option<Certificate>,
    pub exact_work: u64,
    pub driver_cells: usize,
    pub reason: &'static str,
}
/// Identical original weights and world-coordinate poles across the other
/// tensor axis make that coordinate depend on this original axis alone.
/// Exact plane membership at level and strict whole-curve monotonicity then
/// prove that the complete chart preimage is the stated interior UV line.
pub fn certify(
    s: &Surface,
    plane: [[f64; 3]; 3],
    axis: usize,
    level: f64,
    max_work: u64,
    max_driver: usize,
) -> Result<Report> {
    s.validate()?;
    if axis > 1
        || !level.is_finite()
        || plane.iter().flatten().any(|x| !x.is_finite())
        || !(1..=100_000_000).contains(&max_work)
        || max_driver > 100000
    {
        return Err(Error::new(
            "BREP_SOURCE_INTERIOR_FIBER",
            "Choose finite source inputs and bounded proof work",
        ));
    }
    let mut out = Report {
        certificate: None,
        exact_work: 0,
        driver_cells: 0,
        reason: "source-interior-fiber-layout-unproven",
    };
    let counts = [s.control_points.len(), s.control_points[0].len()];
    let degrees = [s.degree_u, s.degree_v];
    let knots = [&s.knots_u, &s.knots_v];
    if s.periodic_u || s.periodic_v || s.control_points[0][0].len() != 3 {
        return Ok(out);
    }
    for a in 0..2 {
        if degrees[a] == 0
            || degrees[a] > 32
            || knots[a][..=degrees[a]]
                .iter()
                .any(|&t| t != knots[a][degrees[a]])
            || knots[a][counts[a]..]
                .iter()
                .any(|&t| t != knots[a][counts[a]])
        {
            return Ok(out);
        }
    }
    let d = [knots[axis][degrees[axis]], knots[axis][counts[axis]]];
    if !(d[0] < level && level < d[1]) {
        return Ok(out);
    }
    let Some(world_axis) = (0..3).find(|&k| plane.iter().all(|p| p[k] == plane[0][k])) else {
        return Ok(out);
    };
    let cost = (counts[0] as u64) * (counts[1] as u64) * 2;
    if cost >= max_work {
        out.reason = "source-interior-fiber-work-limit";
        return Ok(out);
    }
    out.exact_work = cost;
    let pole = |i: usize, j: usize| {
        if axis == 0 {
            (&s.control_points[i][j], s.weights[i][j])
        } else {
            (&s.control_points[j][i], s.weights[j][i])
        }
    };
    for i in 0..counts[axis] {
        let (p, w) = pole(i, 0);
        for j in 1..counts[1 - axis] {
            let (q, v) = pole(i, j);
            if p[world_axis] != q[world_axis] || w != v {
                return Ok(out);
            }
        }
    }
    let c = Curve {
        degree: degrees[axis],
        knots: knots[axis].clone(),
        control_points: (0..counts[axis]).map(|i| pole(i, 0).0.clone()).collect(),
        weights: (0..counts[axis]).map(|i| pole(i, 0).1).collect(),
        periodic: false,
    };
    out.reason = "source-interior-fiber-plane-root-unproven";
    let Some(point) = nurbs_core::curve_surface_plane::verify_curve_point(
        &c,
        plane,
        [level, d[0], d[1]],
        false,
        (max_work - out.exact_work).min(cad_predicates::MAX_WORK),
    )?
    else {
        return Ok(out);
    };
    out.exact_work += point.work_used;
    if point.outcome != cad_predicates::BezierIdentity::Equal {
        return Ok(out);
    }
    out.reason = "source-interior-fiber-uniqueness-unproven";
    if max_driver == 0 {
        return Ok(out);
    }
    let driver = nurbs_core::curve_axis_driver::certify(&c, world_axis, max_driver)?;
    out.driver_cells = driver.visited;
    if !driver.monotonic_proven {
        return Ok(out);
    }
    out.certificate = Some(Certificate {
        surface: s.clone(),
        plane,
        axis,
        level,
    });
    out.reason = "source-interior-fiber-qualified";
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    pub(super) fn fixture(rational: bool) -> Surface {
        let y = if rational {
            [0., 0.5, 1.]
        } else {
            [0., 1., 1.]
        };
        let weights = if rational {
            vec![1., 2., 1.]
        } else {
            vec![1.; 3]
        };
        Surface {
            degree_u: 1,
            degree_v: 2,
            knots_u: vec![0., 0., 1., 1.],
            knots_v: vec![0., 0., 0., 1., 1., 1.],
            control_points: (0..2)
                .map(|z| {
                    (0..3)
                        .map(|i| vec![[1., 1., 0.][i], y[i], z as f64])
                        .collect()
                })
                .collect(),
            weights: vec![weights; 2],
            periodic_u: false,
            periodic_v: false,
        }
    }
    fn plane(y: f64) -> [[f64; 3]; 3] {
        [[0., y, 0.], [1., y, 0.], [0., y, 1.]]
    }
    #[test]
    fn original_polynomial_and_rational_charts_have_exact_unique_interior_fibers() {
        for (rational, level, y) in [(false, 0.625, 0.859375), (true, 0.5, 0.5)] {
            let s = fixture(rational);
            let r = certify(&s, plane(y), 1, level, 100_000_000, 10).unwrap();
            assert!(r.certificate.is_some(), "{}", r.reason);
            assert_eq!(r.driver_cells, 1);
            let fiber = r.certificate.unwrap();
            assert_eq!(fiber.surface(), &s);
            assert_eq!(fiber.coordinate(), (1, level));
            let transposed = Surface {
                degree_u: s.degree_v,
                degree_v: s.degree_u,
                knots_u: s.knots_v.clone(),
                knots_v: s.knots_u.clone(),
                control_points: (0..3)
                    .map(|v| (0..2).map(|u| s.control_points[u][v].clone()).collect())
                    .collect(),
                weights: (0..3)
                    .map(|v| (0..2).map(|u| s.weights[u][v]).collect())
                    .collect(),
                periodic_u: false,
                periodic_v: false,
            };
            assert!(certify(&transposed, plane(y), 0, level, 100_000_000, 10)
                .unwrap()
                .certificate
                .is_some());
            let mut domain = s.clone();
            domain.knots_v = vec![2., 2., 2., 4., 4., 4.];
            assert!(
                certify(&domain, plane(y), 1, 2. + 2. * level, 100_000_000, 10)
                    .unwrap()
                    .certificate
                    .is_some()
            );
            assert!(certify(&s, plane(y + 1e-12), 1, level, 100_000_000, 10)
                .unwrap()
                .certificate
                .is_none());
            assert!(certify(&s, plane(y), 1, level + 1e-12, 100_000_000, 10)
                .unwrap()
                .certificate
                .is_none());
            assert!(certify(&s, plane(y), 1, level, 100_000_000, 0)
                .unwrap()
                .certificate
                .is_none());
            assert!(certify(&s, plane(y), 1, level, 1, 10)
                .unwrap()
                .certificate
                .is_none());
            let mut changed = s.clone();
            changed.control_points[1][1][1] += 1e-12;
            assert!(certify(&changed, plane(y), 1, level, 100_000_000, 10)
                .unwrap()
                .certificate
                .is_none());
            let mut changed = s.clone();
            changed.weights[1][1] += 1e-12;
            assert!(certify(&changed, plane(y), 1, level, 100_000_000, 10)
                .unwrap()
                .certificate
                .is_none());
        }
    }
    #[test]
    fn extra_roots_degenerate_planes_and_boundary_levels_refuse_proof() {
        let mut s = fixture(false);
        for row in &mut s.control_points {
            row[1][1] = 2.;
        }
        // y(t)=4t-3t² meets y=1.25 twice inside the original chart.
        let r = certify(&s, plane(1.25), 1, 0.5, 100_000_000, 100).unwrap();
        assert!(r.certificate.is_none());
        assert!(r.driver_cells > 0);
        let s = fixture(false);
        assert!(
            certify(&s, [[0., 0.859375, 0.]; 3], 1, 0.625, 100_000_000, 10)
                .unwrap()
                .certificate
                .is_none()
        );
        assert!(certify(&s, plane(1.), 1, 1., 100_000_000, 10)
            .unwrap()
            .certificate
            .is_none());
        assert!(certify(&s, plane(f64::NAN), 1, 0.625, 100_000_000, 10).is_err());
    }
}
