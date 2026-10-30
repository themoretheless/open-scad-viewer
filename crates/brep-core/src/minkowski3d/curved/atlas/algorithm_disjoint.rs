//! atlas algorithm: disjoint.
use super::*;
pub fn disjoint(a: &[Net; 2], b: &[Net; 2], depth: u8, work: &mut Work) -> Result<bool> {
    work.charge(1)?;
    let pa = a.iter().flat_map(points).collect::<Vec<_>>();
    let pb = b.iter().flat_map(points).collect::<Vec<_>>();
    if separated(&pa, &pb, work)? {
        return Ok(true);
    }
    if depth == 4 {
        return Ok(false);
    }
    // Split the larger swept box; every child covers its full time interval.
    let diameter = |p: &[P]| {
        (0..3)
            .map(|k| {
                p.iter().map(|p| p[k].clone()).max().unwrap()
                    - p.iter().map(|p| p[k].clone()).min().unwrap()
            })
            .max()
            .unwrap()
    };
    let (first, other, swap) = if diameter(&pa) >= diameter(&pb) {
        (a, b, false)
    } else {
        (b, a, true)
    };
    let x = bernstein::subdivide(&first[0], work)?;
    let y = bernstein::subdivide(&first[1], work)?;
    for (x, y) in x.into_iter().zip(y) {
        let child = [x, y];
        if !(if swap {
            disjoint(other, &child, depth + 1, work)?
        } else {
            disjoint(&child, other, depth + 1, work)?
        }) {
            return Ok(false);
        }
    }
    Ok(true)
}
