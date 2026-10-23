//! Inclusion from a complete original boundary census and one interior witness.
//! The caller must first prove the original region simple with disjoint holes.
use super::*;
use cad_predicates::{
    AuthoredScalar, Limits, PlanarPairOutcome, PlanarReduction, PlanarScalarOutcome,
    PredicateContext, SourceArena, ToleranceContext,
};
fn bezier(c: &Curve) -> bool {
    let n = c.control_points.len();
    let d = c.domain();
    !c.periodic
        && (1..=8).contains(&c.degree)
        && n == c.degree + 1
        && c.knots[..=c.degree].iter().all(|&t| t == d[0])
        && c.knots[n..].iter().all(|&t| t == d[1])
}
fn disjoint_hulls(a: &Curve, b: &Curve) -> bool {
    (0..2).any(|k| {
        let extent = |c: &Curve| {
            c.control_points
                .iter()
                .fold([f64::INFINITY, f64::NEG_INFINITY], |r, p| {
                    [r[0].min(p[k]), r[1].max(p[k])]
                })
        };
        let x = extent(a);
        let y = extent(b);
        x[1] < y[0] || y[1] < x[0]
    })
}
pub(super) struct Proof {
    pub inside: bool,
    pub reason: &'static str,
    pub boundary: Option<usize>,
    pub phase: Option<&'static str>,
}
fn unknown(reason: &'static str, boundary: Option<usize>) -> Proof {
    Proof {
        inside: false,
        reason,
        boundary,
        phase: None,
    }
}
pub(super) fn inside(
    pc: &Curve,
    original: &[Vec<Curve>],
    domain: &FaceDomain,
    cells: &mut usize,
    exact: &mut u64,
    max_cells: usize,
    max_exact: u64,
) -> Result<Proof> {
    if !bezier(pc)
        || original
            .iter()
            .flatten()
            .any(|c| !bezier(c) || c.degree > 3)
    {
        return Ok(unknown("unsupported-basis-or-budget", None));
    }
    if !domain.contains_natural_rectangle(bounds(pc, pc.domain())?) {
        return Ok(unknown("unsupported-basis-or-budget", None));
    }
    for (index, boundary) in original.iter().flatten().enumerate() {
        if *cells == max_cells {
            return Ok(unknown("cell-budget", Some(index)));
        }
        *cells += 1;
        if disjoint_hulls(pc, boundary) {
            continue;
        }
        if nurbs_core::trim_simplicity::prove_endpoint_contacts_only(pc, boundary)? {
            continue;
        }
        if *exact == max_exact {
            return Ok(unknown("unsupported-basis-or-budget", None));
        }
        let values = boundary
            .control_points
            .iter()
            .zip(&boundary.weights)
            .chain(pc.control_points.iter().zip(&pc.weights))
            .flat_map(|(p, w)| [p[0], p[1], *w])
            .map(|v| AuthoredScalar::Binary64Bits(v.to_bits()))
            .collect();
        let source = SourceArena::authored("saved-graft-original-boundary-census", 1, values)
            .map_err(|e| invalid(&e.to_string()))?;
        let refs = |start: usize, n: usize| {
            (start..start + n)
                .map(|i| std::array::from_fn(|k| source.leaf(3 * i + k).unwrap()))
                .collect::<Vec<_>>()
        };
        let a = refs(0, boundary.control_points.len());
        let b = refs(boundary.control_points.len(), pc.control_points.len());
        let tolerance = ToleranceContext::default_valid();
        let mut context = PredicateContext::new(
            &source,
            &tolerance,
            Limits {
                max_work: max_exact - *exact,
                ..Limits::default()
            },
            None,
        );
        let endpoint = |q: [f64; 2], r: Option<[f64; 2]>| {
            let index = if q == [0., 0.] {
                0
            } else if q == [1., 1.] {
                pc.control_points.len() - 1
            } else {
                return false;
            };
            let point = &pc.control_points[index];
            if let Some(t) = r {
                let j = if t == [0., 0.] {
                    0
                } else if t == [1., 1.] {
                    boundary.control_points.len() - 1
                } else {
                    return false;
                };
                *point == boundary.control_points[j]
            } else {
                point == &boundary.control_points[0]
                    || point == boundary.control_points.last().unwrap()
            }
        };
        let mut phase = None;
        let (proven, reason) = if boundary.degree == 1 {
            let result = cad_predicates::planar_bezier_scalar_roots(&mut context, &a, &b, 1e-12)
                .map_err(|e| invalid(&e.to_string()))?;
            *exact += result.work_used;
            match result.outcome {
                PlanarScalarOutcome::Roots {
                    reduction: PlanarReduction::LineCarrier { first: true },
                    roots,
                } => (
                    roots.iter().all(|r| endpoint(r.interval, None)),
                    "line-non-endpoint-root",
                ),
                PlanarScalarOutcome::Roots {
                    reduction:
                        PlanarReduction::CommonCoordinate(_) | PlanarReduction::CommonProjection,
                    roots,
                } => (
                    roots.iter().all(|r| endpoint(r.interval, Some(r.interval))),
                    "line-common-parameter-root",
                ),
                PlanarScalarOutcome::Indeterminate(_) => (false, "line-census-indeterminate"),
                _ => (false, "line-census-unresolved"),
            }
        } else {
            if boundary.degree == 2 {
                let separation =
                    cad_predicates::planar_bezier_endpoint_separation(&mut context, &a, &b)
                        .map_err(|e| invalid(&e.to_string()))?;
                if separation.proven {
                    *exact += separation.work_used;
                    continue;
                }
            }
            let result = cad_predicates::planar_bezier_pair_roots(&mut context, &a, &b, 1e-12)
                .map_err(|e| invalid(&e.to_string()))?;
            *exact += result.work_used;
            phase = Some(result.phase);
            match result.outcome {
                PlanarPairOutcome::Roots(roots) => (
                    roots.iter().all(|r| endpoint(r.second, Some(r.first))),
                    "curve-non-endpoint-root",
                ),
                PlanarPairOutcome::Indeterminate(cad_predicates::Reason::ResourceLimit) => {
                    (false, "curve-census-work-limit")
                }
                PlanarPairOutcome::Indeterminate(cad_predicates::Reason::PrecisionExhausted) => {
                    (false, "curve-census-precision-limit")
                }
                PlanarPairOutcome::Indeterminate(cad_predicates::Reason::MissingProof) => {
                    (false, "curve-census-missing-proof")
                }
                _ => (false, "curve-census-unresolved"),
            }
        };
        if !proven {
            let mut report = unknown(reason, Some(index));
            report.phase = phase;
            return Ok(report);
        }
    }
    // A connected open curve disjoint from every complete region boundary stays
    // in one complement component. This whole-point-box witness selects Inside;
    // it is not used as a substitute for the complete boundary census above.
    if *cells == max_cells {
        return Ok(unknown("unsupported-basis-or-budget", None));
    }
    *cells += 1;
    if *cells == max_cells {
        return Ok(unknown("unsupported-basis-or-budget", None));
    }
    let d = pc.domain();
    let mid = d[0] * 0.5 + d[1] * 0.5;
    let point = bounds(pc, [mid, mid])?;
    let r = domain.classify(point, (max_cells - *cells).min(100000))?;
    *cells += r.cells;
    Ok(Proof {
        inside: r.location == Location::Inside,
        reason: "interior-witness",
        boundary: None,
        phase: None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn full_boundary_census_admits_an_inside_arc_and_refuses_a_hole_crossing_and_exhaustion() {
        let wire = |points: &[[f64; 2]]| {
            (0..points.len())
                .map(|i| {
                    Curve::from_polyline(vec![
                        points[i].to_vec(),
                        points[(i + 1) % points.len()].to_vec(),
                    ])
                    .unwrap()
                })
                .collect::<Vec<_>>()
        };
        let outer = wire(&[
            [0., 0.],
            [1., 0.],
            [1., 0.25],
            [1., 1.],
            [0., 1.],
            [0., 0.25],
        ]);
        let hole = wire(&[[0.4, 0.4], [0.4, 0.6], [0.6, 0.6], [0.6, 0.4]]);
        let model = crate::prism::extrude(&[outer, hole], 0., 1.).unwrap();
        let face = model
            .faces
            .iter()
            .position(|f| {
                f.surface
                    .control_points
                    .iter()
                    .flatten()
                    .all(|p| p[2] == 0.)
            })
            .unwrap();
        let f = &model.faces[face];
        let original = std::iter::once(f.outer)
            .chain(f.holes.iter().copied())
            .map(|w| {
                model.loops[w]
                    .coedges
                    .iter()
                    .map(|c| c.pcurve.clone())
                    .collect::<Vec<_>>()
            })
            .collect::<Vec<_>>();
        let audit =
            nurbs_core::trim_region_audit::inspect(&original, 1e-8, 10000, 10000, 10000).unwrap();
        assert_eq!(audit.valid, Some(true));
        let domain = FaceDomain::new(&model, face, 1e-8).unwrap();
        let curve = Curve {
            degree: 5,
            knots: vec![0.; 6].into_iter().chain(vec![1.; 6]).collect(),
            control_points: (0..6).map(|i| vec![i as f64 / 5., 0.25]).collect(),
            weights: vec![1.; 6],
            periodic: false,
        };
        let mut cells = 0;
        let mut exact = 0;
        let result = inside(
            &curve, &original, &domain, &mut cells, &mut exact, 10000, 1000000,
        )
        .unwrap();
        assert!(result.inside, "{} {:?}", result.reason, result.boundary);
        assert!(cells <= 10000 && exact <= 1000000);
        let mut crossing = curve.clone();
        for p in &mut crossing.control_points[1..5] {
            p[1] = 0.6;
        }
        let result = inside(
            &crossing, &original, &domain, &mut 0, &mut 0, 10000, 1000000,
        )
        .unwrap();
        assert!(!result.inside);
        let result = inside(&curve, &original, &domain, &mut 0, &mut 0, 0, 0).unwrap();
        assert!(!result.inside);
    }
}
