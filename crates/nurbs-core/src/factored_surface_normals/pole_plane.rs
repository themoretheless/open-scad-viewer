//! Uniform exact normal-line limits over a COMPLETE collapsed chart boundary.
//! This proves a tangent-plane limit, not quotient regularity, G2 or embedding.
use super::*;
use crate::surface_join::Boundary;
use cad_predicates::{
    LeafRef, rational_surface_normal_row_line, rational_surface_normal_rows_parallel,
};
pub struct PlaneLimit<'a> {
    surface: &'a Surface,
    boundary: Boundary,
    point: [f64; 3],
    order: usize,
    normal: [[f64; 2]; 3],
    reference: [[f64; 2]; 3],
}
impl PlaneLimit<'_> {
    pub fn surface(&self) -> &Surface {
        self.surface
    }
    pub fn boundary(&self) -> Boundary {
        self.boundary
    }
    pub fn point(&self) -> [f64; 3] {
        self.point
    }
    pub fn factor_order(&self) -> usize {
        self.order
    }
    /// Leading desingularized normal numerator in a common positive scale; not unit normals.
    pub fn normal_bounds(&self) -> [[f64; 2]; 3] {
        self.normal
    }
    /// Enclosure of one original leading coefficient, never a rounded normal candidate.
    pub fn reference_bounds(&self) -> [[f64; 2]; 3] {
        self.reference
    }
}
pub struct PlaneLimitReport<'a> {
    pub certificate: Option<PlaneLimit<'a>>,
    pub work: u64,
    pub reason: &'static str,
}
pub struct PlanePairReport {
    pub same_plane: Option<bool>,
    pub work: u64,
}
fn axes(b: Boundary) -> (usize, bool) {
    match b {
        Boundary::UMin => (0, false),
        Boundary::UMax => (0, true),
        Boundary::VMin => (1, false),
        Boundary::VMax => (1, true),
    }
}
fn view(s: &Surface, b: Boundary) -> Surface {
    let (axis, _) = axes(b);
    if axis == 0 {
        return s.clone();
    }
    Surface {
        degree_u: s.degree_v,
        degree_v: s.degree_u,
        knots_u: s.knots_v.clone(),
        knots_v: s.knots_u.clone(),
        control_points: (0..s.control_points[0].len())
            .map(|v| s.control_points.iter().map(|r| r[v].clone()).collect())
            .collect(),
        weights: (0..s.weights[0].len())
            .map(|v| s.weights.iter().map(|r| r[v]).collect())
            .collect(),
        periodic_u: s.periodic_v,
        periodic_v: s.periodic_u,
    }
}
fn mirrored(s: &Surface, b: Boundary) -> Surface {
    let mut s = view(s, b);
    if axes(b).1 {
        s.control_points.reverse();
        s.weights.reverse();
    }
    s
}
fn arena(surfaces: &[&Surface]) -> (SourceArena, Vec<Vec<Vec<[LeafRef; 4]>>>) {
    let values = surfaces
        .iter()
        .flat_map(|s| {
            s.control_points
                .iter()
                .enumerate()
                .flat_map(move |(u, row)| {
                    row.iter()
                        .enumerate()
                        .flat_map(move |(v, p)| [p[0], p[1], p[2], s.weights[u][v]])
                })
        })
        .map(|x| AuthoredScalar::Binary64Bits(x.to_bits()))
        .collect();
    let arena = SourceArena::authored("original-pole-normal-lines", 1, values).unwrap();
    let mut offset = 0;
    let leaves = surfaces
        .iter()
        .map(|s| {
            s.control_points
                .iter()
                .map(|r| {
                    r.iter()
                        .map(|_| {
                            let values = std::array::from_fn(|k| arena.leaf(offset + k).unwrap());
                            offset += 4;
                            values
                        })
                        .collect()
                })
                .collect()
        })
        .collect();
    (arena, leaves)
}
/// Find the first nonzero original normal row exactly, remove only proved zero
/// powers, and prove that EVERY leading coefficient has the same normal line.
/// A nonzero component bound separates that line uniformly from zero over all
/// boundary parameters. Compactness then gives the same limit along any chart
/// approach to the pole, not only along selected meridians or endpoints.
pub fn prepare_plane_limit(
    s: &Surface,
    boundary: Boundary,
    max_work: u64,
) -> Result<PlaneLimitReport<'_>> {
    s.validate()?;
    check(
        s.control_points[0][0].len() == 3,
        "Pole-plane limits require a 3D original surface",
    )?;
    check(
        (1..=100000000).contains(&max_work),
        "Bound original pole-plane work",
    )?;
    let clamped = |p: usize, k: &[f64], n: usize| {
        (1..=8).contains(&p)
            && n == p + 1
            && k.len() == 2 * (p + 1)
            && k[..=p].iter().all(|x| *x == k[0])
            && k[p + 1..].iter().all(|x| *x == k[p + 1])
    };
    check(
        !s.periodic_u
            && !s.periodic_v
            && clamped(s.degree_u, &s.knots_u, s.control_points.len())
            && clamped(s.degree_v, &s.knots_v, s.control_points[0].len()),
        "Pole-plane limits require an original single clamped Bezier chart",
    )?;
    let mut out = PlaneLimitReport {
        certificate: None,
        work: 0,
        reason: "pole-plane-collapse-unproven",
    };
    let (axis, end) = axes(boundary);
    let original = view(s, boundary);
    let row = &original.control_points[if end {
        original.control_points.len() - 1
    } else {
        0
    }];
    if !row.iter().all(|p| p == &row[0]) {
        return Ok(out);
    }
    let point = [row[0][0], row[0][1], row[0][2]];
    let (prepared, first) = prepare_factor_data(&original, end, 1, max_work)?;
    out.work = prepared.work;
    let Some(prepared) = prepared.certificate else {
        out.reason = prepared.reason;
        return Ok(out);
    };
    let Some(order) = first else {
        out.reason = "pole-plane-normal-identically-zero";
        return Ok(out);
    };
    let mut net = prepared.net;
    // U was already divided out once. Exact signs prove all further removed
    // rows zero; interval scaling uses only small positive integer ratios.
    net.drain(..order - 1);
    let degree = 3 * original.degree_u - 1;
    for (r, row) in net.iter_mut().enumerate() {
        let index = r + order;
        let mut factor = I::point(1.);
        for j in 1..order {
            factor =
                factor.mul(I::point((degree - j) as f64).div(I::point((index - j) as f64))?)?;
        }
        for p in row {
            for x in p {
                *x = x.mul(factor)?;
                if axis == 1 {
                    *x = I::new(-x.hi, -x.lo)?;
                }
            }
        }
    }
    let normal = std::array::from_fn(|k| {
        [
            net[0].iter().map(|p| p[k].lo).fold(f64::INFINITY, f64::min),
            net[0]
                .iter()
                .map(|p| p[k].hi)
                .fold(f64::NEG_INFINITY, f64::max),
        ]
    });
    if !normal.iter().any(|b| b[0] > 0. || b[1] < 0.) {
        out.reason = "pole-plane-nonzero-limit-unproven";
        return Ok(out);
    }
    let source = mirrored(s, boundary);
    let (arena, leaves) = arena(&[&source]);
    let tolerance = ToleranceContext::default_valid();
    let mut query = |column| {
        if out.work == max_work {
            return None;
        }
        let mut ctx = PredicateContext::new(
            &arena,
            &tolerance,
            Limits {
                max_work: (max_work - out.work).min(cad_predicates::MAX_WORK),
                ..Limits::default()
            },
            None,
        );
        let r = rational_surface_normal_row_line(&mut ctx, &leaves[0], order, column).unwrap();
        out.work += r.work_used;
        r.parallel
    };
    let proven = match query(None) {
        Some(value) => value,
        None => {
            let mut complete = true;
            for column in 0..3 * source.degree_v {
                if query(Some(column)) != Some(true) {
                    complete = false;
                    break;
                }
            }
            complete
        }
    };
    if !proven {
        out.reason = if out.work == max_work {
            "pole-plane-work-limit"
        } else {
            "pole-plane-common-line-unproven"
        };
        return Ok(out);
    }
    out.certificate = Some(PlaneLimit {
        surface: s,
        boundary,
        point,
        order,
        normal,
        reference: net[0][0].map(|x| [x.lo, x.hi]),
    });
    out.reason = "pole-plane-uniform-limit-qualified";
    Ok(out)
}
/// Exact agreement of two privately proved limiting planes at the same original
/// pole point. Opposite normal orientations describe the same tangent plane.
pub fn compare_plane_limits(
    a: &PlaneLimit<'_>,
    b: &PlaneLimit<'_>,
    max_work: u64,
) -> Result<PlanePairReport> {
    check(
        (1..=100000000).contains(&max_work),
        "Bound pole-plane comparison work",
    )?;
    check(
        a.point == b.point,
        "Compare plane limits at the same original pole point",
    )?;
    let (a_source, b_source) = (
        mirrored(a.surface, a.boundary),
        mirrored(b.surface, b.boundary),
    );
    let (arena, leaves) = arena(&[&a_source, &b_source]);
    let tolerance = ToleranceContext::default_valid();
    let mut ctx = PredicateContext::new(
        &arena,
        &tolerance,
        Limits {
            max_work: max_work.min(cad_predicates::MAX_WORK),
            ..Limits::default()
        },
        None,
    );
    let r = rational_surface_normal_rows_parallel(
        &mut ctx,
        [&leaves[0], &leaves[1]],
        [a.order, b.order],
    )
    .unwrap();
    Ok(PlanePairReport {
        same_plane: r.parallel,
        work: r.work_used,
    })
}
#[cfg(test)]
mod tests;
