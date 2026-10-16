//! Complete time-pair cover; the fresh whole-carrier theorem covers both UV
//! factors. No sampled stations or boundary-only section proof is used.
use super::*;
use std::collections::BTreeMap;
type Range = [f64; 2];
fn box_for(t: Range) -> Box3 {
    [[0., 1.], [0., 1.], t]
}
fn split(t: Range) -> Option<[Range; 2]> {
    let m = crate::distance_bounds::midpoint(t[0], t[1]);
    (t[0] < m && m < t[1]).then_some([[t[0], m], [m, t[1]]])
}
fn source_value(
    tube: &Tube,
    c: &cell::Context<'_, '_>,
    t: Range,
    out: &mut IdealVolumeReport,
    max: usize,
) -> Result<Option<V3>> {
    if out.cells == max {
        return Ok(None);
    }
    out.cells += 1;
    out.boxes += 1;
    let stage = max.min(out.cells.saturating_add(100000));
    let Some(phase) = c.arc.as_ref().unwrap().phase(t, &mut out.cells, stage)? else {
        return Ok(None);
    };
    let report = crate::sweeps::progressive_miter::vector_certificate::certify_values_traversal(
        c.s.path,
        phase.traversal,
        (max - out.cells).min(100000),
        false,
    )?;
    out.cells += report.cells;
    let Some(value) = report.value else {
        return Ok(None);
    };
    // The freshly proved frame is orthonormal. EVERY point of the complete
    // original section is within rho of C(s), irrespective of frame direction.
    Ok(Some(iv::add(
        decode(value)?,
        [I::new(-tube.radius.hi, tube.radius.hi)?; 3],
    )?))
}
pub(super) fn certify(
    tube: &Tube,
    c: &cell::Context<'_, '_>,
    out: &mut IdealVolumeReport,
    max: usize,
    closed: bool,
) -> Result<()> {
    out.reason = Some("ideal-volume-work-limit");
    let mut values = BTreeMap::new();
    let mut charts = BTreeMap::new();
    let mut local = |pieces: &[Box3], out: &mut IdealVolumeReport| -> Result<bool> {
        let key = pieces
            .iter()
            .map(|b| b[2].map(f64::to_bits))
            .collect::<Vec<_>>();
        if let Some(proved) = charts.get(&key) {
            return Ok(*proved);
        }
        let proved = tube.local(c, pieces, out, max)?;
        if proved || out.cells < max {
            charts.insert(key, proved);
        }
        Ok(proved)
    };
    let mut value = |t: Range, out: &mut IdealVolumeReport| -> Result<Option<V3>> {
        let key = t.map(f64::to_bits);
        if let Some(v) = values.get(&key) {
            return Ok(*v);
        }
        let v = source_value(tube, c, t, out, max)?;
        if v.is_some() || out.cells < max {
            values.insert(key, v);
        }
        Ok(v)
    };
    let root: Range = [0., 1.];
    let mut pairs = vec![(root, root)];
    while let Some((a, b)) = pairs.pop() {
        if out.cells == max {
            return Ok(());
        }
        out.cells += 1;
        let h = [a[0].min(b[0]), a[1].max(b[1])];
        if local(&[box_for(h)], out)? {
            continue;
        }
        if out.cells == max {
            return Ok(());
        }
        if closed && a != b {
            let (early, late) = if a[0] <= b[0] { (a, b) } else { (b, a) };
            if early[1] < late[0] && local(&[box_for([late[0], 1.]), box_for([0., early[1]])], out)?
            {
                continue;
            }
        }
        if out.cells == max {
            return Ok(());
        }
        if a != b {
            if let (Some(x), Some(y)) = (value(a, out)?, value(b, out)?) {
                if (0..3).any(|i| x[i].hi < y[i].lo || y[i].hi < x[i].lo) {
                    continue;
                }
            }
        }
        if out.cells == max {
            return Ok(());
        }
        let split_b = a != b && b[1] - b[0] > a[1] - a[0];
        let Some(children) = split(if split_b { b } else { a }) else {
            out.reason = Some("ideal-volume-atlas-precision-limit");
            return Ok(());
        };
        if a == b {
            // All unordered pairs in the diagonal square; symmetry supplies
            // the reversed cross-pair. No diagonal regularity is skipped.
            pairs.extend([
                (children[0], children[1]),
                (children[1], children[1]),
                (children[0], children[0]),
            ]);
        } else if split_b {
            pairs.extend([(a, children[1]), (a, children[0])]);
        } else {
            pairs.extend([(children[1], b), (children[0], b)]);
        }
    }
    // Only a completed exhaustive pair cover supplies global credit. The
    // compact original carrier x interval/S1 then embeds homeomorphically;
    // restriction to separately valid material trims preserves their topology.
    out.embedding_certified = true;
    out.regularity_certified = true;
    out.parameter_topology_preserved = true;
    out.reason = None;
    Ok(())
}
