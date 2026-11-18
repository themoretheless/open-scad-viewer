//! Full-domain interval subdivision of the retained-minus-linear wall image.
//! V-constant positive weights reduce each U row to a polynomial difference;
//! positive rational U blending preserves its convex-hull norm enclosure.
use super::*;
fn hull(net: &[[Interval; 3]], work: &mut u64, max: u64) -> Result<Option<f64>> {
    let mut upper = 0_f64;
    for point in net {
        let mut squared = Interval::point(0.);
        for delta in point {
            if !charge(work, max) {
                return Ok(None);
            }
            let radius = delta.lo.abs().max(delta.hi.abs());
            squared = squared.add(Interval::point(radius).mul(Interval::point(radius))?)?;
        }
        upper = upper.max(squared.hi.max(0.).sqrt().next_up());
    }
    Ok(Some(upper))
}
fn split(
    net: &[[Interval; 3]],
    work: &mut u64,
    max: u64,
) -> Result<Option<(Vec<[Interval; 3]>, Vec<[Interval; 3]>)>> {
    let mut layer = net.to_vec();
    let mut left = vec![layer[0]];
    let mut right = vec![*layer.last().unwrap()];
    while layer.len() > 1 {
        let mut next = Vec::with_capacity(layer.len() - 1);
        for pair in layer.windows(2) {
            let mut point = [Interval::point(0.); 3];
            for k in 0..3 {
                if !charge(work, max) {
                    return Ok(None);
                }
                point[k] = pair[0][k].add(pair[1][k])?.mul(Interval::point(0.5))?;
            }
            next.push(point);
        }
        left.push(next[0]);
        right.push(*next.last().unwrap());
        layer = next;
    }
    right.reverse();
    Ok(Some((left, right)))
}
pub(super) fn tighten(
    sides: &[Vec<Surface>],
    tolerance: f64,
    work: &mut u64,
    max: u64,
) -> Result<Option<f64>> {
    let mut upper = 0_f64;
    for surface in sides.iter().flatten() {
        let degree = surface.degree_v;
        for row in &surface.control_points {
            let a = &row[0];
            let b = &row[degree];
            let mut net = Vec::with_capacity(row.len());
            for (j, p) in row.iter().enumerate() {
                let t = Interval::point(j as f64).div(Interval::point(degree as f64))?;
                let mut delta = [Interval::point(0.); 3];
                for k in 0..3 {
                    if !charge(work, max) {
                        return Ok(None);
                    }
                    let original = Interval::point(a[k])
                        .mul(Interval::point(1.).sub(t)?)?
                        .add(Interval::point(b[k]).mul(t)?)?;
                    delta[k] = Interval::point(p[k]).sub(original)?;
                }
                net.push(delta);
            }
            let mut pending = vec![(net, 0usize)];
            while let Some((net, depth)) = pending.pop() {
                let Some(bound) = hull(&net, work, max)? else {
                    return Ok(None);
                };
                if bound <= tolerance || depth == 2 {
                    upper = upper.max(bound);
                    continue;
                }
                let Some((left, right)) = split(&net, work, max)? else {
                    return Ok(None);
                };
                pending.push((right, depth + 1));
                pending.push((left, depth + 1));
            }
        }
    }
    Ok(Some(upper))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn subdivision_encloses_entire_bernstein_arch_and_refuses_partial_work() {
        let row = vec![
            vec![0.; 3],
            vec![0.; 3],
            vec![1., 0., 0.],
            vec![0.; 3],
            vec![0.; 3],
            vec![0.; 3],
        ];
        let surface = Surface {
            degree_u: 1,
            degree_v: 5,
            knots_u: vec![0., 0., 1., 1.],
            knots_v: [vec![0.; 6], vec![1.; 6]].concat(),
            control_points: vec![row.clone(), row],
            weights: vec![vec![1.; 6]; 2],
            periodic_u: false,
            periodic_v: false,
        };
        let sides = vec![vec![surface.clone()]];
        let mut work = 0;
        let upper = tighten(&sides, 0.5, &mut work, 10000).unwrap().unwrap();
        assert!(upper >= 0.3456 && upper <= 0.5);
        for j in 0..=100 {
            let point = surface.evaluate(0.5, j as f64 / 100.).unwrap().point;
            assert!(point[0].abs() <= upper);
            assert_eq!(point[1], 0.);
            assert_eq!(point[2], 0.);
        }
        let mut spent = 0;
        assert!(
            tighten(&sides, 0.5, &mut spent, work - 1)
                .unwrap()
                .is_none()
        );
        assert_eq!(spent, work - 1);
    }
}
