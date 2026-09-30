//! Point winding of an exactly represented closed XY chord chain.
//! Boundary/uncertain predicates return an error instead of a guessed fill.
use crate::{Result, check, curve_offset::Segment, curve_offset_diagnostics::orientation, numeric};
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FillRule {
    NonZero,
    EvenOdd,
}
pub fn filled(winding: i32, rule: FillRule) -> bool {
    match rule {
        FillRule::NonZero => winding != 0,
        FillRule::EvenOdd => winding % 2 != 0,
    }
}
/// Sum oriented boundary windings; clockwise holes cancel counterclockwise outers.
pub fn at_loops(loops: &[Vec<Segment>], point: [f64; 2]) -> Result<i32> {
    check(
        !loops.is_empty()
            && loops.len() <= 128
            && loops.iter().map(Vec::len).sum::<usize>() <= 65536,
        "Use at most 128 loops and 65536 edges.",
    )?;
    let mut count = 0;
    for chain in loops {
        count += at(chain, point)?;
    }
    Ok(count)
}
pub fn at(chain: &[Segment], point: [f64; 2]) -> Result<i32> {
    check(
        !chain.is_empty()
            && chain.len() <= 65536
            && point.iter().all(|x| x.is_finite() && x.abs() <= 1e9),
        "Use a bounded closed chain and finite point.",
    )?;
    check(
        chain.iter().all(|e| {
            e.points
                .iter()
                .flatten()
                .all(|x| x.is_finite() && x.abs() <= 1e9)
        }) && chain.windows(2).all(|p| p[0].points[1] == p[1].points[0])
            && chain[0].points[0] == chain.last().unwrap().points[1],
        "Winding requires an exactly connected closed chain.",
    )?;
    let mut count = 0;
    for edge in chain {
        let [a, b] = edge.points;
        numeric(a != b, "Winding chain has a degenerate edge.")?;
        let in_box = (0..2).all(|k| point[k] >= a[k].min(b[k]) && point[k] <= a[k].max(b[k]));
        let upward = a[1] <= point[1] && b[1] > point[1];
        let downward = a[1] > point[1] && b[1] <= point[1];
        if !in_box && !upward && !downward {
            continue;
        }
        let side = orientation(a, b, point)?;
        if in_box {
            numeric(
                matches!(side, Some(-1) | Some(1)),
                "Point lies on the boundary or its side is unresolved.",
            )?;
        }
        if upward || downward {
            numeric(
                matches!(side, Some(-1) | Some(1)),
                "Winding ray side is unresolved.",
            )?;
            if upward && side == Some(1) {
                count += 1;
            } else if downward && side == Some(-1) {
                count -= 1;
            }
        }
    }
    Ok(count)
}
#[cfg(test)]
mod tests {
    use super::*;
    fn chain(points: &[[f64; 2]]) -> Vec<Segment> {
        points
            .windows(2)
            .enumerate()
            .map(|(i, p)| Segment {
                points: [p[0], p[1]],
                domain: [i as f64, (i + 1) as f64],
                error_upper_mm: 0.,
            })
            .collect()
    }
    #[test]
    fn square_reversal_outside_and_boundary_are_distinct() {
        let p = [[0., 0.], [4., 0.], [4., 4.], [0., 4.], [0., 0.]];
        let c = chain(&p);
        assert_eq!(at(&c, [2., 2.]).unwrap(), 1);
        assert_eq!(at(&c, [5., 2.]).unwrap(), 0);
        assert_eq!(
            at(&chain(&p.into_iter().rev().collect::<Vec<_>>()), [2., 2.]).unwrap(),
            -1
        );
        assert!(at(&c, [2., 0.]).is_err());
        assert!(at(&c, [0., 0.]).is_err());
    }
    #[test]
    fn bowtie_lobes_have_opposite_winding() {
        let c = chain(&[[0., 0.], [4., 4.], [0., 4.], [4., 0.], [0., 0.]]);
        let top = at(&c, [2., 3.]).unwrap();
        let bottom = at(&c, [2., 1.]).unwrap();
        assert_eq!(top, 1);
        assert_eq!(bottom, -1);
        assert!(filled(top, FillRule::NonZero) && filled(bottom, FillRule::EvenOdd));
        assert!(at(&c, [2., 2.]).is_err());
    }
    #[test]
    fn double_wound_contour_distinguishes_fill_rules() {
        let c = chain(&[
            [0., 0.],
            [4., 0.],
            [4., 4.],
            [0., 4.],
            [0., 0.],
            [4., 0.],
            [4., 4.],
            [0., 4.],
            [0., 0.],
        ]);
        let winding = at(&c, [2., 2.]).unwrap();
        assert_eq!(winding, 2);
        assert!(filled(winding, FillRule::NonZero));
        assert!(!filled(winding, FillRule::EvenOdd));
    }
    #[test]
    fn oriented_hole_and_island_alternate_the_filled_region() {
        let outer = chain(&[[0., 0.], [10., 0.], [10., 10.], [0., 10.], [0., 0.]]);
        let hole = chain(&[[2., 2.], [2., 8.], [8., 8.], [8., 2.], [2., 2.]]);
        let island = chain(&[[4., 4.], [6., 4.], [6., 6.], [4., 6.], [4., 4.]]);
        let loops = vec![outer, hole, island];
        assert_eq!(at_loops(&loops, [1., 5.]).unwrap(), 1);
        assert_eq!(at_loops(&loops, [3., 5.]).unwrap(), 0);
        assert_eq!(at_loops(&loops, [5., 5.]).unwrap(), 1);
        assert!(at_loops(&loops, [2., 5.]).is_err());
    }
}
