//! Root-excision of artificial subdivision seams, with fresh original proofs.
use super::*;
mod boxes;
use boxes::{contains, disjoint, expanded, subtract};
/// Cell whose Krawczyk image proves one unique root with a fixed crossing sign.
pub(super) struct UniqueCell {
    pub existence: Box3,
    pub image: [I; 3],
    pub sign: i8,
}
pub(super) fn root(
    c: &Source<'_>,
    s: &Surface,
    ids: [usize; 3],
    cell: UniqueCell,
    out: &mut WindowReport,
    max_cells: usize,
) -> Result<WindowRoot> {
    let UniqueCell {
        existence,
        image,
        sign,
    } = cell;
    let mut domain = map_box(existence, [[0., 1.]; 3], image)?;
    let original = [
        [c.knots()[ids[0]], c.knots()[ids[0] + 1]],
        [s.knots_u[ids[1]], s.knots_u[ids[1] + 1]],
        [s.knots_v[ids[2]], s.knots_v[ids[2] + 1]],
    ];
    for _ in 0..8 {
        if out.cells == max_cells {
            break;
        }
        out.cells += 1;
        // The existing root lies in domain. A fresh uniqueness proof on a
        // padded original-span search still owns that same root. Its scale
        // must not collapse with an artificial cell or an already exact axis.
        let search = refinement_box(domain, original);
        match krawczyk(&c.field(s, ids, search)?)? {
            Verdict::Unique(next, next_sign) => {
                if next_sign != sign {
                    return Err(crate::Error::new(
                        "NURBS_ROOT_PROOF_CONTRADICTION",
                        "Window root orientation changed during refinement",
                    ));
                }
                let mut next = map_box(search, [[0., 1.]; 3], next)?;
                for k in 0..3 {
                    next[k][0] = next[k][0].max(domain[k][0]);
                    next[k][1] = next[k][1].min(domain[k][1]);
                    if next[k][0] > next[k][1] {
                        return Err(crate::Error::new(
                            "NURBS_ROOT_PROOF_CONTRADICTION",
                            "Fresh enclosures of the same root are disjoint",
                        ));
                    }
                }
                if next == domain {
                    break;
                }
                domain = next;
            }
            Verdict::Unresolved => break,
            Verdict::Excluded => {
                return Err(crate::Error::new(
                    "NURBS_ROOT_PROOF_CONTRADICTION",
                    "Certified window root excluded during refinement",
                ))
            }
        }
    }
    let p = c.position(ids[0], domain[0])?;
    Ok(WindowRoot {
        spans: ids,
        existence_domain: existence,
        parameter_box: domain,
        position_bounds: std::array::from_fn(|k| [p[k].lo, p[k].hi]),
        crossing_sign: sign,
    })
}
pub(super) fn repair(
    c: &Source<'_>,
    s: &Surface,
    window: Box3,
    max_cells: usize,
    stop_first: bool,
    out: &mut WindowReport,
) -> Result<()> {
    let mut pending = std::collections::VecDeque::from(std::mem::take(&mut out.unresolved));
    let mut retained = Vec::new();
    let mut covers: Vec<(Box3, usize)> = out
        .roots
        .iter()
        .enumerate()
        .map(|(i, r)| (r.existence_domain, i))
        .collect();
    while let Some(cell) = pending.pop_front() {
        if out.cells == max_cells {
            retained.push(cell);
            retained.extend(pending);
            break;
        }
        let mut covered = false;
        for &(domain, _) in &covers {
            if out.cells == max_cells {
                break;
            }
            out.cells += 1;
            if let Some(parts) = subtract(cell, domain) {
                // Never let cover classification create an unbounded pending forest.
                if pending.len() + parts.len() + retained.len() <= max_cells {
                    pending.extend(parts);
                    covered = true;
                }
                break;
            }
        }
        if covered {
            continue;
        }
        if out.cells == max_cells {
            retained.push(cell);
            retained.extend(pending);
            break;
        }
        let Some((ids, proposal)) = expanded(c, s, cell, window) else {
            retained.push(cell);
            continue;
        };
        out.cells += 1;
        match krawczyk(&c.field(s, ids, proposal)?)? {
            Verdict::Excluded => {
                out.excluded += 1;
            } // The proposal contains the entire unresolved cell.
            Verdict::Unresolved => retained.push(cell),
            Verdict::Unique(image, sign) => {
                let enclosure = map_box(proposal, [[0., 1.]; 3], image)?;
                let mut same = None;
                let mut ambiguous = false;
                for &(domain, owner) in &covers {
                    if out.cells == max_cells {
                        ambiguous = true;
                        break;
                    }
                    out.cells += 1;
                    if contains(domain, enclosure) {
                        if same.is_some_and(|i| i != owner) {
                            return Err(crate::Error::new(
                                "NURBS_ROOT_PROOF_CONTRADICTION",
                                "Unique root belongs to two distinct owners",
                            ));
                        }
                        same = Some(owner);
                    }
                }
                for (i, r) in out.roots.iter().enumerate() {
                    if ambiguous || out.cells == max_cells {
                        ambiguous = true;
                        break;
                    }
                    out.cells += 1;
                    if contains(proposal, r.parameter_box) || same == Some(i) {
                        if same.is_some_and(|j| j != i) || r.crossing_sign != sign {
                            return Err(crate::Error::new(
                                "NURBS_ROOT_PROOF_CONTRADICTION",
                                "Unique root identity or orientation changed",
                            ));
                        }
                        same = Some(i);
                    } else if !disjoint(r.parameter_box, enclosure) {
                        ambiguous = true;
                        break;
                    }
                }
                if ambiguous {
                    retained.push(cell);
                    continue;
                }
                let index = if let Some(index) = same {
                    index
                } else {
                    let cell = UniqueCell {
                        existence: proposal,
                        image,
                        sign,
                    };
                    let record = root(c, s, ids, cell, out, max_cells)?;
                    let index = out.roots.len();
                    out.roots.push(record);
                    index
                };
                covers.push((proposal, index));
                // Root identity is proved by containment in a unique existence domain,
                // never by distance between approximate world points or midpoint snapping.
                if stop_first {
                    retained.extend(pending);
                    out.unresolved = retained;
                    return Ok(());
                }
                if pending.len() + retained.len() + 1 > max_cells {
                    retained.push(cell);
                    retained.extend(pending);
                    break;
                }
                pending.push_back(cell);
                pending.extend(retained.drain(..));
            }
        }
    }
    out.unresolved = retained;
    Ok(())
}
