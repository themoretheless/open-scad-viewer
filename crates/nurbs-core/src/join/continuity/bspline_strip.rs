//! Exact original B-spline jets plus independent bounded seam normal covers.
use super::{Boundary, ExactStripJetReport};
use crate::{Result, surface::Surface};
use cad_predicates::{
    AuthoredScalar, BezierIdentity, BsplineStripLocation, Limits, ParameterIdentity,
    PredicateContext, SourceArena, ToleranceContext,
};

#[derive(Clone, Copy)]
struct Location {
    span: usize,
    knot: usize,
    inward: bool,
}
fn ends(s: &Surface, b: Boundary) -> Location {
    let (p, n, k, _) = b.cross(s);
    let span = if b.max {
        (p..n).rev().find(|&i| k[i] < k[i + 1])
    } else {
        (p..n).find(|&i| k[i] < k[i + 1])
    }
    .unwrap();
    Location {
        span,
        knot: if b.max { n } else { p },
        inward: !b.max,
    }
}

fn identity(
    a: &Surface,
    b: &Surface,
    ab: Boundary,
    bb: Boundary,
    locations: [Location; 2],
    order: usize,
    scalar: AuthoredScalar,
    propose_exact_scale: bool,
    max_work: u64,
) -> Result<(BezierIdentity, u64)> {
    let mut values = Vec::new();
    let mut offsets = [0; 2];
    let mut ks = [0; 2];
    for (side, (s, boundary)) in [(a, ab), (b, bb)].into_iter().enumerate() {
        offsets[side] = values.len();
        for i in 0..boundary.along(s).1 {
            for j in 0..boundary.cross(s).1 {
                let (u, v) = if boundary.cross_u { (j, i) } else { (i, j) };
                values.extend_from_slice(&s.control_points[u][v]);
                values.push(s.weights[u][v]);
            }
        }
        ks[side] = values.len();
        values.extend_from_slice(boundary.cross(s).2);
    }
    let si = values.len();
    values.push(1.);
    let mut authored = values
        .into_iter()
        .map(|v| AuthoredScalar::Binary64Bits(v.to_bits()))
        .collect::<Vec<_>>();
    authored[si] = scalar;
    let source = SourceArena::authored("original-bspline-strip-jets", 1, authored)
        .map_err(|e| crate::input(e.to_string()))?;
    let refs = |side: usize, s: &Surface, boundary: Boundary| {
        let n = boundary.cross(s).1;
        (0..boundary.along(s).1)
            .map(|i| {
                (0..n)
                    .map(|j| {
                        std::array::from_fn(|k| {
                            source.leaf(offsets[side] + 4 * (i * n + j) + k).unwrap()
                        })
                    })
                    .collect()
            })
            .collect::<Vec<_>>()
    };
    let a_refs = refs(0, a, ab);
    let b_refs = refs(1, b, bb);
    let k_refs = |side: usize, len: usize| {
        (0..len)
            .map(|i| source.leaf(ks[side] + i).unwrap())
            .collect::<Vec<_>>()
    };
    let ak = k_refs(0, ab.cross(a).2.len());
    let bk = k_refs(1, bb.cross(b).2.len());
    let ls = std::array::from_fn(|i| BsplineStripLocation {
        span: locations[i].span,
        parameter: source.leaf(ks[i] + locations[i].knot).unwrap(),
        inward_positive: locations[i].inward,
    });
    let tolerance = ToleranceContext::default_valid();
    let mut ctx = PredicateContext::new(
        &source,
        &tolerance,
        Limits {
            max_work,
            ..Limits::default()
        },
        None,
    );
    let proof = cad_predicates::rational_bspline_projective_strip_jet_identity(
        &mut ctx,
        [&a_refs, &b_refs],
        [&ak, &bk],
        [ab.cross(a).0, bb.cross(b).0],
        ls,
        source.leaf(si).unwrap(),
        order,
        propose_exact_scale,
    )
    .map_err(|e| crate::input(e.to_string()))?;
    Ok((proof.outcome, proof.work_used))
}

fn same_basis(
    a: &Surface,
    b: &Surface,
    ab: Boundary,
    bb: Boundary,
    max_work: u64,
) -> Result<(bool, u64)> {
    let aa = ab.along(a);
    let ba = bb.along(b);
    if aa.0 != ba.0 || aa.1 != ba.1 || aa.2.len() != ba.2.len() {
        return Ok((false, 0));
    }
    if aa.2 == ba.2 {
        return Ok((true, 0));
    }
    let source = SourceArena::authored(
        "bspline-along-basis-map",
        1,
        aa.2.iter()
            .chain(ba.2)
            .map(|v| AuthoredScalar::Binary64Bits(v.to_bits()))
            .collect(),
    )
    .map_err(|e| crate::input(e.to_string()))?;
    let ak = (0..aa.2.len())
        .map(|i| source.leaf(i).unwrap())
        .collect::<Vec<_>>();
    let bk = (0..ba.2.len())
        .map(|i| source.leaf(aa.2.len() + i).unwrap())
        .collect::<Vec<_>>();
    let tolerance = ToleranceContext::default_valid();
    let mut ctx = PredicateContext::new(
        &source,
        &tolerance,
        Limits {
            max_work,
            ..Limits::default()
        },
        None,
    );
    let proof = cad_predicates::affine_bspline_knot_basis_identity(&mut ctx, [&ak, &bk], aa.0)
        .map_err(|e| crate::input(e.to_string()))?;
    Ok((proof.outcome == ParameterIdentity::Equal, proof.work_used))
}

// Original span jets enclose every parameter, including one-sided knot limits.
// The prior 4096-cell / depth-12 ceilings are SHARED across a source's covers.
fn normal(s: &Surface, b: Boundary, l: Location, cells: &mut usize) -> bool {
    let (p, n, k, _) = b.along(s);
    let point = b.cross(s).2[l.knot];
    let mut pending = (p..n)
        .filter(|&i| k[i] < k[i + 1])
        .map(|i| (i, [k[i], k[i + 1]], 0usize))
        .collect::<Vec<_>>();
    while let Some((along, domain, depth)) = pending.pop() {
        if *cells >= 4096 {
            return false;
        }
        *cells += 1;
        let (span, rectangle) = if b.cross_u {
            ([l.span, along], [[point, point], domain])
        } else {
            ([along, l.span], [domain, [point, point]])
        };
        let Ok(j) = crate::surface_measure::first_jets::calculate(s, span, rectangle) else {
            return false;
        };
        let mut regular = false;
        for axis in 0..3 {
            let a = (axis + 1) % 3;
            let b = (axis + 2) % 3;
            let Ok(x) = j[1][a]
                .mul(j[2][b])
                .and_then(|x| x.sub(j[1][b].mul(j[2][a])?))
            else {
                return false;
            };
            regular |= x.lo > 0. || x.hi < 0.;
        }
        if regular {
            continue;
        }
        let mid = domain[0] * 0.5 + domain[1] * 0.5;
        if depth >= 12 || !(domain[0] < mid && mid < domain[1]) {
            return false;
        }
        pending.push((along, [mid, domain[1]], depth + 1));
        pending.push((along, [domain[0], mid], depth + 1));
    }
    true
}

// Multiplicity guarantees interior Ck where applicable. Every exceptional
// original knot and every declared periodic closure needs an independent
// exact whole-strip identity AND regularity; a periodic flag grants no credit.
fn source_joins(
    s: &Surface,
    order: usize,
    work: &mut u64,
    max_work: u64,
    cells: &mut usize,
) -> Result<bool> {
    for cross_u in [true, false] {
        let b = Boundary {
            cross_u,
            max: false,
        };
        let (p, n, k, periodic) = b.cross(s);
        let spans = (p..n).filter(|&i| k[i] < k[i + 1]).collect::<Vec<_>>();
        let mut pairs = Vec::new();
        for pair in spans.windows(2) {
            let knot = pair[0] + 1;
            let mult = k.iter().filter(|&&v| v == k[knot]).count();
            if p >= order && mult <= p - order {
                continue;
            }
            pairs.push([
                Location {
                    span: pair[0],
                    knot,
                    inward: false,
                },
                Location {
                    span: pair[1],
                    knot,
                    inward: true,
                },
            ]);
        }
        if periodic {
            pairs.push([
                Location {
                    span: *spans.last().unwrap(),
                    knot: n,
                    inward: false,
                },
                Location {
                    span: spans[0],
                    knot: p,
                    inward: true,
                },
            ]);
        }
        for pair in pairs {
            let (proof, used) = identity(
                s,
                s,
                b,
                b,
                pair,
                order,
                AuthoredScalar::Binary64Bits(1f64.to_bits()),
                true,
                max_work - *work,
            )?;
            *work += used;
            if proof != BezierIdentity::Equal
                || !normal(s, b, pair[0], cells)
                || !normal(s, b, pair[1], cells)
            {
                return Ok(false);
            }
        }
    }
    Ok(true)
}

pub(super) fn inspect(
    a: &Surface,
    b: &Surface,
    ab: Boundary,
    bb: Boundary,
    order: usize,
    scale: f64,
    max_work: u64,
    scalar: Option<AuthoredScalar>,
) -> Result<ExactStripJetReport> {
    let mut out = ExactStripJetReport {
        certified: false,
        exact_identity: false,
        regularity_certified: false,
        work: 0,
        reason: "unsupported-common-bspline-basis",
    };
    if [a, b]
        .iter()
        .any(|s| s.control_points.len() > 33 || s.control_points[0].len() > 33)
    {
        return Ok(out);
    }
    let (common, used) = same_basis(a, b, ab, bb, max_work)?;
    out.work += used;
    if !common {
        return Ok(out);
    }
    let mut cells = [0; 2];
    for (i, s) in [a, b].into_iter().enumerate() {
        // A self-seam has one original surface, not two independent sources.
        // Reuse only an identical complete definition, never sampled geometry
        // or a caller's certificate. Its normal cover keeps ONE shared ledger.
        if i == 1 && a == b {
            continue;
        }
        if !source_joins(s, order, &mut out.work, max_work, &mut cells[i])? {
            out.reason = "original-bspline-internal-or-periodic-jets-unproved";
            return Ok(out);
        }
    }
    let locations = [ends(a, ab), ends(b, bb)];
    let (proof, used) = identity(
        a,
        b,
        ab,
        bb,
        locations,
        order,
        scalar.unwrap_or(AuthoredScalar::Binary64Bits(scale.to_bits())),
        false,
        max_work - out.work,
    )?;
    out.work += used;
    out.exact_identity = proof == BezierIdentity::Equal;
    out.reason = match proof {
        BezierIdentity::Equal => "bspline-seam-regularity-unproved",
        BezierIdentity::Different => "bspline-projective-jet-relation-different",
        BezierIdentity::Indeterminate(_) => "exact-bspline-jet-work-unresolved",
    };
    if out.exact_identity {
        out.regularity_certified = normal(a, ab, locations[0], &mut cells[0])
            && if a == b {
                normal(b, bb, locations[1], &mut cells[0])
            } else {
                normal(b, bb, locations[1], &mut cells[1])
            };
        out.certified = out.regularity_certified;
        if out.certified {
            out.reason = "exact-regular-original-bspline-strip-jets";
        }
    }
    Ok(out)
}
#[cfg(test)]
mod tests;
