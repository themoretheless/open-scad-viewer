//! Original rational affine fibers with strictly ordered heights, including
//! singular end parameterizations. No trim validity, boundary Gk or Solid credit.
use crate::affine_volume::RationalPiece;
use crate::{Result, check, distance_bounds::Interval as I, surface::Surface};
use cad_predicates::{AuthoredScalar, PredicateContext, SourceArena, ToleranceContext};

#[derive(Clone, Copy, Debug)]
pub struct Proposal {
    pub source_normal: [f64; 3],
    /// Two world fiber coordinates, followed by the world height covector.
    pub world_rows: [[f64; 3]; 3],
}
#[derive(Clone, Copy, Debug)]
pub struct Limits {
    pub cells: usize,
    pub exact_work: u64,
}
#[derive(Clone, Debug)]
pub struct FramePiece {
    pub piece: usize,
    pub cells: usize,
    pub restricted_determinant_abs_lower: f64,
}
#[derive(Clone, Debug)]
pub struct Report {
    pub embedding_certified: bool,
    /// Relative to a separately validated region inside the natural UV rectangle.
    /// Trim validity is not inferred.
    pub parameter_topology_preserved: bool,
    /// On open knot/time pieces. Not a closed-cube Jacobian certificate.
    pub spanwise_interior_regular: bool,
    pub height_direction: Option<i8>,
    pub endpoint_height_flat_orders: Option<[usize; 2]>,
    pub known_singular_parameter_ends: [bool; 2],
    pub cells: usize,
    pub exact_work: u64,
    pub carrier: Option<crate::surface_injectivity::Report>,
    pub frames: Vec<FramePiece>,
    pub reason: Option<&'static str>,
}
fn determinant(a: [[I; 3]; 3]) -> Result<I> {
    a[0][0]
        .mul(a[1][1].mul(a[2][2])?.sub(a[1][2].mul(a[2][1])?)?)?
        .sub(a[0][1].mul(a[1][0].mul(a[2][2])?.sub(a[1][2].mul(a[2][0])?)?)?)?
        .add(a[0][2].mul(a[1][0].mul(a[2][1])?.sub(a[1][1].mul(a[2][0])?)?)?)
}
fn failure(mut report: Report, reason: &'static str) -> Report {
    report.reason = Some(reason);
    report
}
/// F(u,v,t)=T(t)+A(t)(Q(u,v)-origin), with equal normalized time pieces.
/// Original coefficient identities place Q in one source plane and every fiber
/// in an ordered world height plane. Each rational height law is strictly
/// monotone even when its endpoint derivatives vanish. Global carrier
/// injectivity and whole-piece frame-rank covers then make F globally injective.
/// Compactness gives a material homeomorphism. Positive interior height
/// derivatives give nonzero Jacobians on open source knot/time pieces only.
/// The actual/original boundary and all G1/G2 joins remain owner obligations.
pub fn certify(
    carrier: &Surface,
    origin: [f64; 3],
    pieces: &[RationalPiece],
    p: Proposal,
    l: Limits,
) -> Result<Report> {
    carrier.validate()?;
    check(
        carrier.control_points.iter().map(Vec::len).sum::<usize>() <= 10000
            && carrier
                .control_points
                .iter()
                .flatten()
                .all(|q| q.len() == 3 && q.iter().all(|v| v.is_finite() && v.abs() <= 1e9))
            && (1..=256).contains(&pieces.len())
            && pieces.iter().all(RationalPiece::valid)
            && origin
                .iter()
                .chain(p.source_normal.iter())
                .chain(p.world_rows.iter().flatten())
                .all(|v| v.is_finite() && v.abs() <= 1e9)
            && p.source_normal.iter().any(|v| *v != 0.)
            && p.world_rows[2].iter().any(|v| *v != 0.)
            && l.cells <= 100000
            && l.exact_work <= cad_predicates::MAX_WORK,
        "Invalid original ordered fiber volume or limits",
    )?;
    let mut r = Report {
        embedding_certified: false,
        parameter_topology_preserved: false,
        spanwise_interior_regular: false,
        height_direction: None,
        endpoint_height_flat_orders: None,
        known_singular_parameter_ends: [false; 2],
        cells: 0,
        exact_work: 0,
        carrier: None,
        frames: Vec::new(),
        reason: None,
    };
    // ALL raw input validation precedes every budget/unsupported refusal.
    let mut values = Vec::new();
    values.extend(p.source_normal);
    values.extend(origin);
    values.extend(p.world_rows.iter().flatten());
    for q in carrier.control_points.iter().flatten() {
        values.extend(q);
    }
    let motion = values.len();
    for piece in pieces {
        for (pose, w) in piece.controls.iter().zip(&piece.weights) {
            values.extend(pose.iter().flatten());
            values.push(*w);
        }
    }
    let arena = SourceArena::authored(
        "original-ordered-affine-fiber-volume",
        1,
        values
            .into_iter()
            .map(|v| AuthoredScalar::Binary64Bits(v.to_bits()))
            .collect(),
    )
    .map_err(|e| crate::input(e.to_string()))?;
    let leaf = |i| arena.leaf(i).unwrap();
    let normal = std::array::from_fn(|k| leaf(k));
    let origin_refs = std::array::from_fn(|k| leaf(3 + k));
    let world = std::array::from_fn(|i| std::array::from_fn(|j| leaf(6 + i * 3 + j)));
    let controls = (15..motion)
        .step_by(3)
        .map(|i| std::array::from_fn(|j| leaf(i + j)))
        .collect::<Vec<_>>();
    let mut cursor = motion;
    let poses = pieces
        .iter()
        .map(|piece| {
            piece
                .controls
                .iter()
                .map(|_| {
                    let refs = std::array::from_fn(|j| leaf(cursor + j));
                    cursor += 13;
                    refs
                })
                .collect::<Vec<_>>()
        })
        .collect::<Vec<_>>();
    let tolerance = ToleranceContext::default_valid();
    let mut context = PredicateContext::new(
        &arena,
        &tolerance,
        cad_predicates::Limits {
            max_work: l.exact_work,
            ..Default::default()
        },
        None,
    );
    let proof = cad_predicates::binary_fibered_affine_premises(
        &mut context,
        &controls,
        origin_refs,
        normal,
        world,
        &poses,
    )
    .map_err(|e| crate::input(e.to_string()))?;
    r.exact_work = proof.work_used;
    let Some(direction) = proof.direction else {
        return Ok(failure(r, "fiber-coefficient-premises-unproved"));
    };
    r.height_direction = Some(if direction == cad_predicates::Sign::Positive {
        1
    } else {
        -1
    });
    r.endpoint_height_flat_orders = proof.endpoint_flat_orders;
    r.known_singular_parameter_ends = proof.endpoint_flat_orders.unwrap().map(|order| order > 0);
    if l.cells == 0 {
        return Ok(failure(r, "fiber-volume-cell-limit"));
    }
    let cover = crate::surface_injectivity::certify(carrier, l.cells)?;
    r.cells += cover.spans;
    let proven = cover.proven;
    r.carrier = Some(cover);
    if !proven {
        return Ok(failure(r, "fiber-carrier-global-injectivity-unproved"));
    }
    let mut orientation = None;
    for (index, piece) in pieces.iter().enumerate() {
        let mut pending = vec![[0., 1.]];
        let mut cells = 0;
        let mut lower = f64::INFINITY;
        while let Some(domain) = pending.pop() {
            if r.cells == l.cells {
                return Ok(failure(r, "fiber-volume-cell-limit"));
            }
            r.cells += 1;
            cells += 1;
            let pose =
                crate::affine_volume::original_rational_pose(piece, I::new(domain[0], domain[1])?)?;
            let mut a = [[I::point(0.); 3]; 3];
            for row in 0..2 {
                for col in 0..3 {
                    for k in 0..3 {
                        a[row][col] = a[row][col]
                            .add(I::point(p.world_rows[row][k]).mul(pose.linear[col][0][k])?)?;
                    }
                }
            }
            a[2] = p.source_normal.map(I::point);
            let det = determinant(a)?;
            let sign = if det.lo > 0. {
                Some(1)
            } else if det.hi < 0. {
                Some(-1)
            } else {
                None
            };
            if let Some(sign) = sign {
                if orientation.is_some_and(|s| s != sign) {
                    return Ok(failure(r, "fiber-frame-rank-unproved"));
                }
                orientation = Some(sign);
                lower = lower.min(if sign > 0 { det.lo } else { -det.hi });
                continue;
            }
            let mid = crate::distance_bounds::midpoint(domain[0], domain[1]);
            if mid <= domain[0] || mid >= domain[1] {
                return Ok(failure(r, "fiber-frame-precision-limit"));
            }
            pending.extend([[mid, domain[1]], [domain[0], mid]]);
        }
        r.frames.push(FramePiece {
            piece: index,
            cells,
            restricted_determinant_abs_lower: lower,
        });
    }
    r.embedding_certified = true;
    r.parameter_topology_preserved = true;
    r.spanwise_interior_regular = true;
    Ok(r)
}
#[cfg(test)]
mod tests;
