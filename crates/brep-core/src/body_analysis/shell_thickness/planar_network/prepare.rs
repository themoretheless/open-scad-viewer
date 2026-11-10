//! Original straight/curved-boundary T witnesses propose source cuts.
//! Rounded cuts and endpoint coupling are proposals; every saved child keeps
//! fresh whole-original curve agreement. Original cap carriers are unchanged.
use super::*;
use num_rational::BigRational as Q;
use num_traits::{ToPrimitive, Zero};
use nurbs_core::curve_partition_agreement as partition;
use std::collections::BTreeMap;
mod curved;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CutKind {
    ExactStraight,
    CurvedWitness,
}
#[derive(Clone, Debug, PartialEq)]
pub struct Cut {
    pub kind: CutKind,
    pub patch: usize,
    pub side: usize,
    /// Exact straight inverse, or literal rational representation of the
    /// curved parameter witness. Kind distinguishes the two authorities.
    pub parameter: Q,
    pub stored_parameter: f64,
    pub witness_patch: usize,
    pub witness_corner: usize,
    /// Original curve-to-witness distance at `parameter`: the exact rational
    /// inverse for ExactStraight, or the stored scalar for CurvedWitness.
    /// This does not bound straight-root rounding at `stored_parameter`;
    /// full post-coupling original-curve admission handles that separately.
    pub source_distance_upper: f64,
    /// Native curved-search and witness-proof cells; straight inverse is algebraic.
    pub discovery_cells: usize,
}
#[derive(Clone)]
pub(in crate::body_analysis::shell_thickness) struct Piece {
    pub address: Boundary,
    pub curve: Curve,
    pub pcurve: Curve,
}
pub(in crate::body_analysis::shell_thickness) struct Prepared {
    pub pieces: Vec<Vec<Piece>>,
    pub cuts: Vec<Cut>,
}
fn q(v: f64) -> Q {
    Q::from_float(v).unwrap()
}
fn corner(s: &Surface, i: usize) -> &[f64] {
    let n = s.control_points.len() - 1;
    let m = s.control_points[0].len() - 1;
    match i {
        0 => &s.control_points[0][0],
        1 => &s.control_points[n][0],
        2 => &s.control_points[n][m],
        _ => &s.control_points[0][m],
    }
}
fn scalar(v: &Q) -> Result<f64> {
    v.to_f64()
        .filter(|v| v.is_finite())
        .ok_or_else(|| refused("Prepared boundary clock overflow"))
}
fn allowance(max: usize, cells: usize) -> Result<usize> {
    max.checked_sub(cells)
        .filter(|n| *n > 0)
        .ok_or_else(|| refused("Shared original boundary partition proof allowance exhausted"))
}
fn proposals(
    sources: &[Surface],
    tolerance: f64,
    max: usize,
    cells: &mut usize,
) -> Result<Vec<Cut>> {
    let mut cuts = Vec::new();
    for (patch, s) in sources.iter().enumerate() {
        for side in 0..4 {
            let curve = network::boundary(s, side).0;
            if curve.degree != 1 {
                cuts.extend(curved::cuts(
                    sources, patch, side, &curve, tolerance, max, cells,
                )?);
                continue;
            }
            let a = &curve.control_points[0];
            let b = &curve.control_points[1];
            let Some(axis) = (0..3).find(|&k| a[k] != b[k]) else {
                continue;
            };
            let d = curve.domain();
            let mut candidates = BTreeMap::<Q, (usize, usize)>::new();
            for (other, target) in sources.iter().enumerate() {
                if other == patch {
                    continue;
                }
                for i in 0..4 {
                    let p = corner(target, i);
                    let f = (q(p[axis]) - q(a[axis])) / (q(b[axis]) - q(a[axis]));
                    if f <= Q::zero() || f >= Q::from_integer(1.into()) {
                        continue;
                    }
                    if (0..3).all(|k| q(a[k]) + &f * (q(b[k]) - q(a[k])) == q(p[k])) {
                        let rate = &f * q(curve.weights[0])
                            / (q(curve.weights[1]) * (Q::from_integer(1.into()) - &f)
                                + q(curve.weights[0]) * &f);
                        candidates
                            .entry(q(d[0]) + rate * (q(d[1]) - q(d[0])))
                            .or_insert((other, i));
                    }
                }
            }
            if candidates.len() > 63 {
                return Err(refused("Original boundary needs more than 63 T cuts"));
            }
            let mut previous = None;
            for (parameter, (other, i)) in candidates {
                let stored_parameter = scalar(&parameter)?;
                if stored_parameter <= d[0]
                    || stored_parameter >= d[1]
                    || previous == Some(stored_parameter)
                {
                    return Err(refused(
                        "Distinct original T vertices cannot share a rounded stored clock",
                    ));
                }
                previous = Some(stored_parameter);
                cuts.push(Cut {
                    kind: CutKind::ExactStraight,
                    source_distance_upper: 0.,
                    discovery_cells: 0,
                    patch,
                    side,
                    parameter,
                    stored_parameter,
                    witness_patch: other,
                    witness_corner: i,
                });
            }
        }
    }
    Ok(cuts)
}
pub(in crate::body_analysis::shell_thickness) fn build(
    sources: &[Surface],
    original_cuts: Option<&[Cut]>,
    tolerance: f64,
    max: usize,
    cells: &mut usize,
) -> Result<Prepared> {
    let cuts = if let Some(cuts) = original_cuts {
        cuts.to_vec()
    } else {
        proposals(sources, tolerance, max, cells)?
    };
    let mut all = Vec::new();
    for (patch, s) in sources.iter().enumerate() {
        let mut prepared = Vec::new();
        for side in 0..4 {
            let (original, pc) = network::boundary(s, side);
            let d = original.domain();
            let pd = pc.domain();
            let local = cuts
                .iter()
                .filter(|c| c.patch == patch && c.side == side)
                .collect::<Vec<_>>();
            if local.is_empty() {
                prepared.push(Piece {
                    address: Boundary {
                        patch,
                        side,
                        parameters: d,
                    },
                    curve: original,
                    pcurve: pc,
                });
                continue;
            }
            let parameters = local.iter().map(|c| c.stored_parameter).collect::<Vec<_>>();
            let mut world =
                partition::partition(&original, &parameters, tolerance, allowance(max, *cells)?)?;
            *cells += world.visited;
            if !world.geometry_preserved || !world.endpoint_joins_exact {
                return Err(refused("Original boundary partition remains unproven"));
            }
            let pc_cuts = parameters
                .iter()
                .map(|v| {
                    scalar(
                        &(q(pd[0])
                            + (q(*v) - q(d[0])) * (q(pd[1]) - q(pd[0])) / (q(d[1]) - q(d[0]))),
                    )
                })
                .collect::<Result<Vec<_>>>()?;
            let mut uv = partition::partition(&pc, &pc_cuts, 1e-8, allowance(max, *cells)?)?;
            *cells += uv.visited;
            if !uv.geometry_preserved || !uv.endpoint_joins_exact {
                return Err(refused("Original UV boundary partition remains unproven"));
            }
            // Retain the EXACT original natural UV side as an image. This
            // prevents tiny constant-coordinate lerp drift from trimming away
            // original carrier points used by the cap-distance theorem.
            let fixed = (0..2)
                .find(|&k| pc.control_points[0][k] == pc.control_points[1][k])
                .unwrap();
            let varying = 1 - fixed;
            let a = pc.control_points[0][varying];
            let b = pc.control_points[1][varying];
            for piece in &mut uv.pieces {
                for point in &mut piece.curve.control_points {
                    point[fixed] = pc.control_points[0][fixed];
                }
            }
            uv.pieces[0].curve.control_points[0] = pc.control_points[0].clone();
            *uv.pieces
                .last_mut()
                .unwrap()
                .curve
                .control_points
                .last_mut()
                .unwrap() = pc.control_points[1].clone();
            for piece in &uv.pieces {
                let p = &piece.curve.control_points;
                if (p[1][varying] > p[0][varying]) != (b > a)
                    || p[0][varying] == p[1][varying]
                    || p.iter()
                        .any(|p| p[varying] < a.min(b) || p[varying] > a.max(b))
                {
                    return Err(refused(
                        "Prepared UV segment collapses or leaves its original natural side",
                    ));
                }
            }
            for (i, cut) in local.iter().enumerate() {
                let p = corner(&sources[cut.witness_patch], cut.witness_corner).to_vec();
                *world.pieces[i].curve.control_points.last_mut().unwrap() = p.clone();
                world.pieces[i + 1].curve.control_points[0] = p;
            }
            for (mut w, p) in world.pieces.into_iter().zip(uv.pieces) {
                let pc_proof = partition::certify(&pc, &p.curve, 1e-8, allowance(max, *cells)?)?;
                *cells += pc_proof.visited;
                if !pc_proof.agreement_proven {
                    return Err(refused(
                        "Actual original-side UV partition remains unproven",
                    ));
                }
                let proof =
                    partition::certify(&original, &w.curve, tolerance, allowance(max, *cells)?)?;
                *cells += proof.visited;
                if !proof.agreement_proven {
                    return Err(refused(
                        "Coupled T vertex versus complete original boundary remains unproven",
                    ));
                }
                w.agreement = Some(proof);
                prepared.push(Piece {
                    address: Boundary {
                        patch,
                        side,
                        parameters: w.curve.domain(),
                    },
                    curve: w.curve,
                    pcurve: p.curve,
                });
            }
        }
        all.push(prepared);
    }
    Ok(Prepared { pieces: all, cuts })
}
