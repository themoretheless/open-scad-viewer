//! Degree-one rational interpolation between two projected monotone rails.
//! The interpolation fraction is generally coupled in both UV parameters.
use super::*;

pub(crate) fn certify(s: &Surface, work: &mut usize, limit: usize) -> Option<[usize; 2]> {
    let degree = [s.degree_u, s.degree_v];
    if s.periodic_u
        || s.periodic_v
        || degree.iter().any(|d| !(1..=32).contains(d))
        || !degree.contains(&1)
        || s.control_points.len() != degree[0] + 1
        || s.control_points[0].len() != degree[1] + 1
    {
        return None;
    }
    let mut charge = || {
        if *work >= limit {
            return None;
        }
        *work += 1;
        Some(())
    };
    for (knots, d) in [(&s.knots_u, degree[0]), (&s.knots_v, degree[1])] {
        charge()?;
        if knots
            .iter()
            .enumerate()
            .any(|(i, k)| *k != knots[if i <= d { 0 } else { d + 1 }])
        {
            return None;
        }
    }
    for across in (0..2).filter(|&axis| degree[axis] == 1) {
        let along = degree[1 - across];
        let pole = |rail: usize, i: usize, k: usize| {
            if across == 0 {
                s.control_points[rail][i][k]
            } else {
                s.control_points[i][rail][k]
            }
        };
        for transverse in 0..3 {
            charge()?;
            let left = pole(0, 0, transverse);
            let right = pole(1, 0, transverse);
            if left == right {
                continue;
            }
            let mut constant = true;
            for i in 1..=along {
                charge()?;
                if pole(0, i, transverse) != left || pole(1, i, transverse) != right {
                    constant = false;
                    break;
                }
            }
            if !constant {
                continue;
            }
            for height in (0..3).filter(|&k| k != transverse) {
                charge()?;
                let increasing = pole(0, along, height) > pole(0, 0, height);
                let mut ordered = true;
                for i in 1..=along {
                    charge()?;
                    if !(0..2).all(|rail| {
                        let a = pole(rail, i - 1, height);
                        let b = pole(rail, i, height);
                        if increasing { a < b } else { a > b }
                    }) {
                        ordered = false;
                        break;
                    }
                }
                if ordered {
                    // For rail denominators W0,W1>0 and normalized across
                    // parameter u, lambda=uW1/((1-u)W0+uW1) has lambda_u>0.
                    // The transverse coordinate recovers lambda. At fixed
                    // lambda, height is a convex blend of the rail heights,
                    // with the same strict derivative sign, including at
                    // the endpoints (strict ordered rational Bezier poles).
                    // Thus height recovers v, then lambda recovers u. In
                    // (lambda,v) the projected Jacobian is triangular and
                    // full rank, proving the original same-point rank too.
                    return Some(if across == 0 {
                        [transverse, height]
                    } else {
                        [height, transverse]
                    });
                }
            }
        }
    }
    None
}
