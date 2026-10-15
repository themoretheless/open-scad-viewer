//! `action: "profile"`: merges the normalized fill regions into one bounded,
//! nonzero planar profile so hosts do not re-run the ring booleans in JS.
use super::*;

/// Even-odd regions are re-oriented by nesting depth and unioned; nonzero
/// regions use the planar nonzero rule. All regions are then unioned and
/// bounded by `mesh_section::bounded_svg_contours`.
pub(super) fn merge_profile(regions: Vec<(Rings, FillRule)>) -> Result<Rings> {
    let mut sections = Vec::with_capacity(regions.len());
    for (mut rings, rule) in regions {
        rings.retain(|r| r.len() >= 3);
        if rings.is_empty() {
            sections.push(Vec::new());
            continue;
        }
        if matches!(rule, FillRule::EvenOdd) {
            let copy = rings.clone();
            for (i, r) in rings.iter_mut().enumerate() {
                let p = r[0];
                let mut depth = 0;
                for (j, other) in copy.iter().enumerate() {
                    if i == j {
                        continue;
                    }
                    let mut inside = false;
                    for k in 0..other.len() {
                        let a = other[k];
                        let b = other[(k + 1) % other.len()];
                        if (a[1] > p[1]) != (b[1] > p[1])
                            && p[0] < (b[0] - a[0]) * (p[1] - a[1]) / (b[1] - a[1]) + a[0]
                        {
                            inside = !inside;
                        }
                    }
                    if inside {
                        depth += 1;
                    }
                }
                if (rings::area(r) > 0.) != (depth % 2 == 0) {
                    r.reverse();
                }
            }
            sections.push(rings::planar(&rings, &vec![], "union")?);
        } else {
            sections.push(rings::nonzero(&rings)?);
        }
    }
    let merged = match sections.split_first() {
        None => Vec::new(),
        Some((first, rest)) => {
            let mut acc = first.clone();
            for next in rest {
                acc = rings::planar(&acc, next, "union")?;
            }
            acc
        }
    };
    Ok(mesh_section::bounded_svg_contours(merged)?)
}
