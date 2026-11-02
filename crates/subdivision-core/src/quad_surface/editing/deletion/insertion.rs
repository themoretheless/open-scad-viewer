//! Retain original interior controls by splitting admitted projected quads.
//! Candidate choices do not grant authority: the saved active disk is replayed
//! after every insertion. No source coordinate is moved or silently discarded.
use super::*;
use projection::Projector;
fn inside(p: &mut Projector, q: [usize; 4], x: usize) -> Result<bool> {
    // Strict membership excludes every quad edge, including projected overlaps.
    for i in 0..4 {
        let (a, b) = (q[i], q[(i + 1) % 4]);
        if p.orient(a, b, x)? == 0 && p.between(a, x, b)? {
            return Ok(false);
        }
    }
    for [a, b, c, d] in [q, [q[1], q[2], q[3], q[0]]] {
        let u = p.orient(a, b, c)?;
        let v = p.orient(a, c, d)?;
        if u < 0 || v < 0 || u + v == 0 {
            continue;
        }
        for (area, [a, b, c]) in [(u, [a, b, c]), (v, [a, c, d])] {
            if area > 0
                && p.orient(a, b, x)? >= 0
                && p.orient(b, c, x)? >= 0
                && p.orient(c, a, x)? >= 0
            {
                return Ok(true);
            }
        }
    }
    Ok(false)
}
pub(super) fn insert(
    p: &mut Projector,
    points: &[[f64; 3]],
    faces: &mut Vec<[usize; 4]>,
    forbidden: &BTreeSet<(usize, usize)>,
    work: &mut usize,
    max: usize,
) -> Result<()> {
    for x in p.boundary_count()..points.len() {
        let mut admitted = None;
        for f in 0..faces.len() {
            if !inside(p, faces[f], x)? {
                continue;
            }
            for q in [
                faces[f],
                [faces[f][1], faces[f][2], faces[f][3], faces[f][0]],
            ] {
                if *work == max {
                    return Err(error(
                        "Interior-control reconstruction candidate allowance exhausted",
                    ));
                }
                *work += 1;
                let a = [q[0], q[1], q[2], x];
                let b = [q[2], q[3], q[0], x];
                if !p.quad(a)? || !p.quad(b)? {
                    continue;
                }
                let mut proposal = faces.clone();
                proposal[f] = a;
                proposal.push(b);
                // All original points, including pending controls, remain in
                // the projection arena; no chord may hide a pending control.
                if tiling::verify(p, &points[..=x], &proposal, forbidden).is_ok() {
                    admitted = Some(proposal);
                    break;
                }
            }
            if admitted.is_some() {
                break;
            }
        }
        *faces = admitted.ok_or_else(|| {
            error("No bounded quad insertion retains every original interior control")
        })?;
    }
    Ok(())
}
