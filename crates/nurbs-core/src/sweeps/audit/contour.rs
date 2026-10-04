//! Continuous planar contour ownership. This proves a cap domain only, not
//! swept-shell containment or the geometry of a subsequently constructed cap.
use crate::{
    Result, check,
    curve::Curve,
    curve_distance,
    trim_domain::{Location, TrimDomain},
    trim_simplicity,
};
#[derive(Clone, Debug)]
pub struct Report {
    pub cap_domain_certified: bool,
    pub plane_axis: Option<usize>,
    pub pairs: usize,
    pub cells: usize,
    pub reason: Option<&'static str>,
}
/// Fully segmented clamped Bezier data needs no knot insertion: copying
/// existing coefficients preserves the represented rational curve exactly.
fn exact_segments(c: &Curve) -> Option<Vec<Curve>> {
    let n = c.control_points.len();
    if c.degree == 0 {
        return None;
    }
    if c.periodic {
        return crate::exact_curve_segments::inspect(c);
    }
    if c.knots[..=c.degree].iter().any(|k| *k != c.knots[c.degree])
        || c.knots[n..].iter().any(|k| *k != c.knots[n])
    {
        return crate::exact_curve_segments::inspect(c);
    }
    for knot in &c.knots[c.degree + 1..n] {
        if c.knots.iter().filter(|k| *k == knot).count() != c.degree {
            return crate::exact_curve_segments::inspect(c);
        }
    }
    Some(
        crate::certificates::audit::curve_span_domains(c)
            .into_iter()
            .map(|(i, _)| Curve {
                degree: c.degree,
                knots: std::iter::repeat_n(c.knots[i], c.degree + 1)
                    .chain(std::iter::repeat_n(c.knots[i + 1], c.degree + 1))
                    .collect(),
                control_points: c.control_points[i - c.degree..=i].to_vec(),
                weights: c.weights[i - c.degree..=i].to_vec(),
                periodic: false,
            })
            .collect(),
    )
}
pub fn inspect(
    loops: &[Vec<Curve>],
    tolerance: f64,
    max_pairs: usize,
    max_cells: usize,
) -> Result<Report> {
    check(
        (1..=16).contains(&loops.len())
            && loops.iter().all(|l| !l.is_empty())
            && loops.iter().map(Vec::len).sum::<usize>() <= 64
            && tolerance.is_finite()
            && tolerance > 0.
            && max_pairs <= 100000
            && max_cells <= 100000,
        "Invalid contour audit input/budget",
    )?;
    for curve in loops.iter().flatten() {
        curve.validate()?;
        check(
            curve.control_points.iter().all(|p| p.len() == 3),
            "Contour audit needs 3D curves",
        )?;
    }
    let mut out = Report {
        cap_domain_certified: false,
        plane_axis: None,
        pairs: 0,
        cells: 0,
        reason: Some("planar-contours-unproved"),
    };
    let origin = &loops[0][0].control_points[0];
    let Some(axis) = (0..3).find(|axis| {
        loops
            .iter()
            .flatten()
            .flat_map(|c| &c.control_points)
            .all(|p| p[*axis] == origin[*axis])
    }) else {
        return Ok(out);
    };
    out.plane_axis = Some(axis);
    let mut projected = Vec::with_capacity(loops.len());
    let mut segments = 0;
    for contour in loops {
        let mut uv_loop = Vec::new();
        for curve in contour {
            let Some(pieces) = exact_segments(curve) else {
                out.reason = Some("contour-decomposition-unproved");
                return Ok(out);
            };
            segments += pieces.len();
            if segments > 256 {
                out.reason = Some("contour-segment-budget-exhausted");
                return Ok(out);
            }
            for mut piece in pieces {
                piece.control_points = piece
                    .control_points
                    .iter()
                    .map(|p| (0..3).filter(|i| *i != axis).map(|i| p[i]).collect())
                    .collect();
                uv_loop.push(piece);
            }
        }
        projected.push(uv_loop);
    }
    for contour in &projected {
        if out.pairs == max_pairs || out.cells == max_cells {
            out.reason = Some("contour-budget-exhausted");
            return Ok(out);
        }
        if contour.len() < 2 {
            out.reason = Some("contour-simplicity-unproved");
            return Ok(out);
        }
        let r = trim_simplicity::inspect(
            contour,
            tolerance,
            max_pairs - out.pairs,
            (max_cells - out.cells).min(256),
        )?;
        out.pairs += r.pairs.len();
        out.cells += r.cells;
        if !r.proven_simple {
            out.reason = Some("contour-simplicity-unproved");
            return Ok(out);
        }
    }
    // Disjoint simple boundaries plus one certified winding query establish
    // containment of the complete connected loop, not just the query point.
    for a in 0..projected.len() {
        for b in a + 1..projected.len() {
            for ca in &projected[a] {
                for cb in &projected[b] {
                    if out.pairs == max_pairs || out.cells == max_cells {
                        out.reason = Some("contour-budget-exhausted");
                        return Ok(out);
                    }
                    out.pairs += 1;
                    let hull = |c: &Curve| -> Vec<crate::distance_bounds::Interval> {
                        (0..2)
                            .map(|axis| crate::distance_bounds::Interval {
                                lo: c
                                    .control_points
                                    .iter()
                                    .map(|p| p[axis])
                                    .fold(f64::INFINITY, f64::min),
                                hi: c
                                    .control_points
                                    .iter()
                                    .map(|p| p[axis])
                                    .fold(f64::NEG_INFINITY, f64::max),
                            })
                            .collect()
                    };
                    out.cells += 1;
                    if crate::distance_bounds::box_distance(&hull(ca), &hull(cb))?.0 > 0. {
                        continue;
                    }
                    if out.cells == max_cells {
                        out.reason = Some("contour-budget-exhausted");
                        return Ok(out);
                    }
                    let r = curve_distance::distance(
                        ca,
                        cb,
                        tolerance,
                        (max_cells - out.cells).min(256),
                    )?;
                    out.cells += r.cells;
                    if r.distance_interval_mm[0] <= 0. {
                        out.reason = Some("contour-separation-unproved");
                        return Ok(out);
                    }
                }
            }
            for (container, query, expected) in if a == 0 {
                vec![(a, b, Location::Inside)]
            } else {
                vec![(a, b, Location::Outside), (b, a, Location::Outside)]
            } {
                if out.cells == max_cells {
                    out.reason = Some("contour-budget-exhausted");
                    return Ok(out);
                }
                let domain = TrimDomain::new(&[projected[container].clone()], tolerance)?;
                let point = &projected[query][0].control_points[0];
                let r = domain.classify_point([point[0], point[1]], max_cells - out.cells)?;
                out.cells += r.cells;
                if r.location != expected {
                    out.reason = Some(if r.location == Location::Unresolved {
                        "contour-winding-unproved"
                    } else {
                        "contour-ownership-violated"
                    });
                    return Ok(out);
                }
            }
        }
    }
    out.cap_domain_certified = true;
    out.reason = None;
    Ok(out)
}
#[cfg(test)]
mod tests {
    use super::*;
    fn square(x: f64, y: f64, size: f64) -> Vec<Curve> {
        let points = [
            [x, y, 0.],
            [x + size, y, 0.],
            [x + size, y + size, 0.],
            [x, y + size, 0.],
        ];
        (0..4)
            .map(|i| Curve {
                degree: 1,
                knots: vec![0., 0., 1., 1.],
                control_points: vec![points[i].to_vec(), points[(i + 1) % 4].to_vec()],
                weights: vec![1., 2.],
                periodic: false,
            })
            .collect()
    }
    #[test]
    fn continuous_outer_hole_ownership_requires_simple_separated_contours() {
        let loops = [square(0., 0., 10.), square(1., 1., 2.), square(6., 6., 2.)];
        assert!(
            inspect(&loops, 1e-6, 10000, 10000)
                .unwrap()
                .cap_domain_certified
        );
        for loops in [
            vec![square(0., 0., 10.), square(11., 1., 2.)],
            vec![square(0., 0., 10.), square(1., 1., 4.), square(2., 2., 1.)],
            vec![square(0., 0., 10.), square(0., 1., 2.)],
        ] {
            assert!(
                !inspect(&loops, 1e-6, 10000, 10000)
                    .unwrap()
                    .cap_domain_certified
            );
        }
        assert!(
            !inspect(&loops, 1e-6, 0, 10000)
                .unwrap()
                .cap_domain_certified
        );
        assert!(
            !inspect(&loops, 1e-6, 10000, 0)
                .unwrap()
                .cap_domain_certified
        );
    }
    #[test]
    fn nonplanar_and_crossed_contours_never_promote_cap_domain() {
        let mut nonplanar = square(0., 0., 10.);
        nonplanar[0].control_points[0][2] = 1.;
        let r = inspect(&[nonplanar], 1e-6, 1000, 1000).unwrap();
        assert!(!r.cap_domain_certified && r.plane_axis.is_none());
        let mut crossed = square(0., 0., 10.);
        let points = [[0., 0., 0.], [10., 10., 0.], [0., 10., 0.], [10., 0., 0.]];
        for (i, c) in crossed.iter_mut().enumerate() {
            c.control_points = vec![points[i].to_vec(), points[(i + 1) % 4].to_vec()];
        }
        assert!(
            !inspect(&[crossed], 1e-6, 1000, 1000)
                .unwrap()
                .cap_domain_certified
        );
        let r = inspect(&[square(0., 0., 10.), square(1., 1., 2.)], 1e-6, 1, 1).unwrap();
        assert!(!r.cap_domain_certified && r.cells <= 1 && r.pairs <= 1);
    }
    #[cfg(feature = "transport")]
    #[test]
    fn json_transport_retains_scope_and_budget_refusals() {
        use value_codec::json;
        let request = json!({"op":"sweep_contour_audit",
            "loops":[square(0.,0.,10.),square(1.,1.,2.)],
            "tolerance":1e-6,"maxPairs":10000,"maxCells":10000});
        let r = crate::transport::dispatch(request.clone()).unwrap();
        assert_eq!(r["capDomainCertified"], json!(true));
        assert_eq!(r["capGeometryCertified"], json!(false));
        assert_eq!(r["globalEmbeddingCertified"], json!(false));
        for budget in ["maxPairs", "maxCells"] {
            let mut exhausted = request.clone();
            exhausted[budget] = json!(0);
            let r = crate::transport::dispatch(exhausted).unwrap();
            assert_eq!(r["capDomainCertified"], json!(false));
            assert_eq!(r["reason"], json!("contour-budget-exhausted"));
        }
        let mut invalid = request;
        invalid["tolerance"] = json!(-1.);
        assert!(crate::transport::dispatch(invalid).is_err());
    }

    #[test]
    fn segmented_circle_coefficients_transfer_exactly_into_domain_audit() {
        let outer = crate::primitives::circle([0., 0., 0.], [0., 0., 1.], 10.).unwrap();
        let inner = crate::primitives::circle([0., 0., 0.], [0., 0., 1.], 2.).unwrap();
        let pieces = exact_segments(&outer).unwrap();
        assert_eq!(pieces.len(), 4);
        for (index, piece) in pieces.iter().enumerate() {
            assert_eq!(
                piece.control_points,
                outer.control_points[index * 2..index * 2 + 3]
            );
            assert_eq!(piece.weights, outer.weights[index * 2..index * 2 + 3]);
        }
        let r = inspect(&[vec![outer.clone()], vec![inner]], 1e-4, 10000, 10000).unwrap();
        assert!(r.cap_domain_certified, "{r:?}");
        let mut unresolved = outer;
        unresolved.knots = vec![0., 0., 0., 0.1, 0.2, 0.3, 0.4, 0.5, 0.6, 1., 1., 1.];
        // Unsegmented representations require a separate rounding certificate.
        unresolved.validate().unwrap();
        assert!(exact_segments(&unresolved).is_none());
    }
}
