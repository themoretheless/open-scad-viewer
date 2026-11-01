//! Shared exact original tensor queries. Never rounded extracted controls.
use crate::surface::Surface;
use cad_predicates::{
    AuthoredScalar, BezierIdentity, Limits, PredicateContext, RationalTensorSource, SourceArena,
    ToleranceContext,
};
pub(crate) struct Work {
    pub cells: usize,
    pub exact_work: u64,
}
pub(crate) fn query(
    caps: &[&Surface],
    order: Option<usize>,
    out: &mut Work,
    max_cells: usize,
    max_work: u64,
) -> bool {
    if out.exact_work >= max_work
        || caps.iter().any(|s| {
            s.periodic_u
                || s.periodic_v
                || !(1..=8).contains(&s.degree_u)
                || !(1..=8).contains(&s.degree_v)
                || s.control_points.len() > 64
                || s.control_points[0].len() > 64
        })
    {
        return false;
    }
    let charge = caps
        .iter()
        .map(|s| {
            s.control_points.len() * s.control_points[0].len() + s.knots_u.len() + s.knots_v.len()
        })
        .sum::<usize>();
    if charge > 100000 || charge > max_cells.saturating_sub(out.cells) {
        return false;
    }
    out.cells += charge;
    let mut values = Vec::new();
    let mut starts = Vec::new();
    for s in caps {
        let controls = values.len();
        for (p, &w) in s
            .control_points
            .iter()
            .flatten()
            .zip(s.weights.iter().flatten())
        {
            values.extend([p[0], p[1], p[2], w]);
        }
        let u = values.len();
        values.extend_from_slice(&s.knots_u);
        let v = values.len();
        values.extend_from_slice(&s.knots_v);
        starts.push([controls, u, v]);
    }
    let Ok(arena) = SourceArena::authored(
        "original-normalized-filled-tensor-identity",
        1,
        values
            .into_iter()
            .map(|x| AuthoredScalar::Binary64Bits(x.to_bits()))
            .collect(),
    ) else {
        return false;
    };
    let mut controls = Vec::new();
    let mut knots = Vec::new();
    for (s, start) in caps.into_iter().zip(starts) {
        controls.push(
            (0..s.control_points.len())
                .map(|i| {
                    (0..s.control_points[0].len())
                        .map(|j| {
                            std::array::from_fn(|k| {
                                arena
                                    .leaf(start[0] + 4 * (i * s.control_points[0].len() + j) + k)
                                    .unwrap()
                            })
                        })
                        .collect()
                })
                .collect::<Vec<Vec<_>>>(),
        );
        knots.push([
            (0..s.knots_u.len())
                .map(|i| arena.leaf(start[1] + i).unwrap())
                .collect::<Vec<_>>(),
            (0..s.knots_v.len())
                .map(|i| arena.leaf(start[2] + i).unwrap())
                .collect(),
        ]);
    }
    let source = |i: usize| RationalTensorSource {
        degrees: [caps[i].degree_u, caps[i].degree_v],
        controls: &controls[i],
        knots: [&knots[i][0], &knots[i][1]],
    };
    let tolerance = ToleranceContext::default_valid();
    let mut ctx = PredicateContext::new(
        &arena,
        &tolerance,
        Limits {
            max_work: (max_work - out.exact_work).min(cad_predicates::MAX_WORK),
            ..Default::default()
        },
        None,
    );
    let proof = if let Some(order) = order {
        cad_predicates::rational_bspline_tensor_knot_continuity(&mut ctx, source(0), order)
    } else {
        cad_predicates::rational_bspline_tensor_normalized_identity(&mut ctx, source(0), source(1))
    };
    let equal = matches!(proof.map(|r| r.outcome), Ok(BezierIdentity::Equal));
    out.exact_work += ctx.work_used();
    equal
}
