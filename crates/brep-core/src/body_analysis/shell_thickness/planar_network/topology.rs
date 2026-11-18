//! Source complete-boundary discovery: endpoints only index candidates; literal
//! normalized Bezier ownership or fresh exact curve/lift identity decides joins.
use super::*;
use std::collections::{BTreeMap, BTreeSet};
fn point(c: &Curve, end: bool) -> [f64; 3] {
    let p = if end {
        c.control_points.last().unwrap()
    } else {
        &c.control_points[0]
    };
    [p[0], p[1], p[2]]
}
fn key(p: [f64; 3]) -> [u64; 3] {
    p.map(|v| if v == 0. { 0 } else { v.to_bits() })
}
struct Edge {
    curve: Curve,
    uses: Vec<Boundary>,
}
pub(in crate::body_analysis::shell_thickness) fn boundaries(
    sources: &[Surface],
    reversed: &[bool],
    max: u64,
    work: &mut u64,
) -> Result<Vec<Boundary>> {
    discover(sources, reversed, max, work, None)
}
/// Prepared source boundaries with the cell budget and tolerance their
/// pcurve lifts consume.
pub(in crate::body_analysis::shell_thickness) struct PreparedLift<'a> {
    pub prepared: &'a prepare::Prepared,
    pub max_cells: usize,
    pub cells: &'a mut usize,
    pub tolerance: f64,
}
pub(in crate::body_analysis::shell_thickness) fn boundaries_prepared(
    sources: &[Surface],
    reversed: &[bool],
    max: u64,
    work: &mut u64,
    lift: PreparedLift<'_>,
) -> Result<Vec<Boundary>> {
    discover(sources, reversed, max, work, Some(lift))
}
fn discover(
    sources: &[Surface],
    reversed: &[bool],
    max: u64,
    work: &mut u64,
    lift: Option<PreparedLift<'_>>,
) -> Result<Vec<Boundary>> {
    let mut unused_cells = 0;
    let (prepared, max_cells, cells, tolerance) = match lift {
        Some(PreparedLift {
            prepared,
            max_cells,
            cells,
            tolerance,
        }) => (Some(prepared), max_cells, cells, tolerance),
        None => (None, 0, &mut unused_cells, 0.),
    };
    let mut edges = Vec::<Edge>::new();
    let mut buckets = BTreeMap::<[[u64; 3]; 2], Vec<usize>>::new();
    for (patch, s) in sources.iter().enumerate() {
        let pieces = if let Some(p) = prepared {
            p.pieces[patch].clone()
        } else {
            (0..4)
                .map(|side| {
                    let (curve, pcurve) = network::boundary(s, side);
                    prepare::Piece {
                        address: Boundary {
                            patch,
                            side,
                            parameters: curve.domain(),
                        },
                        curve,
                        pcurve,
                    }
                })
                .collect()
        };
        for piece in pieces {
            let address = piece.address;
            let curve = piece.curve;
            let pcurve = piece.pcurve;
            let a = key(point(&curve, false));
            let b = key(point(&curve, true));
            if a == b {
                return Err(refused(
                    "A source boundary has coincident endpoints; explicit pole/cyclic decomposition is required",
                ));
            }
            let pair = if a < b { [a, b] } else { [b, a] };
            let mut chosen = None;
            for &edge in buckets.get(&pair).into_iter().flatten() {
                let e = &edges[edge];
                let reverse = key(point(&e.curve, false)) != a;
                let cost = 4 * (curve.control_points.len() + e.curve.control_points.len()) as u64;
                if cost > max - *work {
                    return Err(refused(
                        "Original source-boundary ownership allowance exhausted",
                    ));
                }
                *work += cost;
                let candidate_pc = if prepared.is_some() {
                    network::prepared_clock(&e.curve, &curve, &pcurve, reverse)?
                } else {
                    pcurve.clone()
                };
                let same = if network::same_owned_boundary(&e.curve, &curve, reverse) {
                    true
                } else {
                    let proof = nurbs_core::curve_surface_agreement::verify_exact(
                        &e.curve,
                        &candidate_pc,
                        s,
                        reverse,
                        max - *work,
                    )?;
                    if let Some(proof) = proof {
                        if proof.work_used > max - *work {
                            return Err(refused("Source exact correspondence exceeds allowance"));
                        }
                        *work += proof.work_used;
                        match proof.outcome {
                            cad_predicates::BezierIdentity::Equal => true,
                            cad_predicates::BezierIdentity::Different if prepared.is_none() => {
                                false
                            }
                            _ if prepared.is_some() => network::prepared_lift(
                                &e.curve,
                                &candidate_pc,
                                s,
                                reverse,
                                tolerance,
                                max_cells,
                                cells,
                            )?,
                            _ => {
                                return Err(refused(
                                    "Original source-boundary equality remains unresolved",
                                ));
                            }
                        }
                    } else if prepared.is_some() {
                        network::prepared_lift(
                            &e.curve,
                            &candidate_pc,
                            s,
                            reverse,
                            tolerance,
                            max_cells,
                            cells,
                        )?
                    } else {
                        return Err(refused(
                            "Original source-boundary correspondence remains unproven",
                        ));
                    }
                };
                if same {
                    if e.uses.len() != 1 {
                        return Err(refused("More than two original patches share a boundary"));
                    }
                    if reversed[e.uses[0].patch] == (reverse ^ reversed[patch]) {
                        return Err(refused(
                            "Original shared sheet boundary traversal is not opposite",
                        ));
                    }
                    chosen = Some(edge);
                    break;
                }
            }
            let edge = chosen.unwrap_or_else(|| {
                let id = edges.len();
                edges.push(Edge {
                    curve,
                    uses: Vec::new(),
                });
                buckets.entry(pair).or_default().push(id);
                id
            });
            edges[edge].uses.push(address);
        }
    }
    let mut neighbors = vec![Vec::new(); sources.len()];
    let mut exposed = Vec::new();
    for e in edges {
        if e.uses.len() == 1 {
            exposed.extend(e.uses);
        } else {
            let a = e.uses[0].patch;
            let b = e.uses[1].patch;
            neighbors[a].push(b);
            neighbors[b].push(a);
        }
    }
    let mut visited = BTreeSet::new();
    let mut pending = vec![0];
    while let Some(p) = pending.pop() {
        if visited.insert(p) {
            pending.extend(neighbors[p].iter().copied());
        }
    }
    if visited.len() != sources.len() {
        return Err(refused(
            "Original patch sheet must be one connected boundary-owned network",
        ));
    }
    if exposed.is_empty() {
        return Err(refused("Planar patch sheet has no exposed boundary"));
    }
    Ok(exposed)
}

// Endpoint pairs only select an additional whole-use proof route. Equal
// endpoints never declare sharing; prepared_clock + original lifts must prove it.
pub(in crate::body_analysis::shell_thickness) fn has_clock_candidates(prepared: &prepare::Prepared) -> bool {
    let mut buckets = BTreeMap::<[[u64; 3]; 2], Vec<&Curve>>::new();
    for piece in prepared.pieces.iter().flatten() {
        let a = key(point(&piece.curve, false));
        let b = key(point(&piece.curve, true));
        let pair = if a < b { [a, b] } else { [b, a] };
        for other in buckets.get(&pair).into_iter().flatten() {
            let reverse = key(point(other, false)) != a;
            if network::needs_prepared(other, &piece.curve, reverse) {
                return true;
            }
        }
        buckets.entry(pair).or_default().push(&piece.curve);
    }
    false
}
