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
fn source_center(
    c: &cell::Context<'_, '_>,
    t: Range,
    refine: bool,
    out: &mut IdealVolumeReport,
    max: usize,
) -> Result<Option<V3>> {
    if out.cells == max {
        return Ok(None);
    }
    out.cells += 1;
    out.boxes += 1;
    let stage = max.min(out.cells.saturating_add(100000));
    let map = c.arc.as_ref().unwrap();
    let traversal = if refine {
        map.phase(t, &mut out.cells, stage)?.map(|p| p.traversal)
    } else {
        map.traversal_bracket(t, &mut out.cells, stage)?
    };
    let Some(traversal) = traversal else {
        return Ok(None);
    };
    let report = crate::sweeps::progressive_miter::vector_certificate::certify_values_traversal(
        c.s.path,
        traversal,
        (max - out.cells).min(100000),
        false,
    )?;
    out.cells += report.cells;
    let Some(value) = report.value else {
        return Ok(None);
    };
    // Keep the path-center enclosure. A sphere predicate preserves its
    // Euclidean separation; expanding to axis cubes would discard that proof.
    Ok(Some(decode(value)?))
}
/// Each complete original section is inside its certified Euclidean ball.
/// A lower distance between center boxes must exceed BOTH upper radii. A
/// midpoint distance, axis-cube overlap or pointwise observation is insufficient.
fn separated_section_balls(a: V3, b: V3, radius: I) -> Result<bool> {
    let lower = crate::distance_bounds::box_distance(&a, &b)?.0;
    Ok(lower > radius.add(radius)?.hi)
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
        let mut proved = tube.local(c, pieces, out, max)?;
        // The closed physical owner deliberately rejects wide lifts as a
        // search proposal. Do not immediately spend an expensive UV metric
        // attempt on the same wide lift; refine its complete pair cover first.
        let narrow = !closed || pieces.iter().map(|b| b[2][1] - b[2][0]).sum::<f64>() <= 0.125;
        if !proved && narrow && tube.transverse_carrier && tube.extended_laws && out.cells < max {
            // A containing physical disk can include a focal point outside
            // the actual carrier. Its failure must not suppress a successful
            // original UV-chart proof. Both cover exactly the same source
            // pairs, including the diagonal and the closed seam.
            proved = super::super::atlas::local_source(c, pieces, out, max)?;
        }
        if proved || out.cells < max {
            charts.insert(key, proved);
        }
        Ok(proved)
    };
    let mut value = |t: Range, refine: bool, out: &mut IdealVolumeReport| -> Result<Option<V3>> {
        let key = (t.map(f64::to_bits), refine);
        if let Some(v) = values.get(&key) {
            return Ok(*v);
        }
        let v = source_center(c, t, refine, out, max)?;
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
        if a != b {
            // Certified balls enclose every original section in either carrier
            // mode. Resolve separated pairs before an expensive joint metric
            // trial. Diagonal pairs still require their full regularity proof.
            if let (Some(x), Some(y)) = (
                value(a, tube.transverse_carrier, out)?,
                value(b, tube.transverse_carrier, out)?,
            ) {
                if separated_section_balls(x, y, tube.radius)? {
                    continue;
                }
            }
        }
        let h = [a[0].min(b[0]), a[1].max(b[1])];
        if local(&[box_for(h)], out)? {
            continue;
        }
        // This pair visit is already paid. Even at the exact limit, an
        // existing wrapped chart or two cached source boxes may finish it
        // without any new source work. Uncached queries guard their own
        // ledger before arithmetic; failure still reaches the limit gate
        // below, and the next pair always requires its own paid visit.
        if closed && a != b {
            let (early, late) = if a[0] <= b[0] { (a, b) } else { (b, a) };
            if early[1] < late[0] && local(&[box_for([late[0], 1.]), box_for([0., early[1]])], out)?
            {
                continue;
            }
        }
        // A node bracket can keep a nonvanishing center uncertainty as
        // pair boxes shrink. For small unresolved spatial pairs, retry
        // separation with the original integral inverse enclosure.
        if a != b && !tube.transverse_carrier && (a[1] - a[0]).max(b[1] - b[0]) <= 1. / 64. {
            if let (Some(x), Some(y)) = (value(a, true, out)?, value(b, true, out)?) {
                if separated_section_balls(x, y, tube.radius)? {
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

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn diagonal_section_balls_use_complete_center_boxes_and_strict_distance() {
        let origin = [0., 0., 0.].map(I::point);
        let diagonal = [1.5, 1.5, 0.].map(I::point);
        let radius = I::point(1.);
        // The axis cubes overlap in every coordinate, but the two full balls
        // are separated: sqrt(1.5^2+1.5^2)>2. This is a whole-section proof.
        assert!(separated_section_balls(origin, diagonal, radius).unwrap());
        assert!(!separated_section_balls(origin, [2., 0., 0.].map(I::point), radius).unwrap());
        assert!(!separated_section_balls(origin, [1., 1., 0.].map(I::point), radius).unwrap());
        let uncertain = [
            I::new(1., 2.).unwrap(),
            I::new(1., 2.).unwrap(),
            I::point(0.),
        ];
        // Its midpoint is separated, but allowed centers at (1,1,0) are not.
        assert!(!separated_section_balls(origin, uncertain, radius).unwrap());
    }
}
