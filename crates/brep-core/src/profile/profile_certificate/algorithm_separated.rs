//! profile certificate algorithm: separated.
use super::*;
pub fn separated(a: &Piece, b: &Piece, n: &[Q; 2], shared: bool) -> bool {
    let project = |p: &[Q; 2]| &p[0] * &n[0] + &p[1] * &n[1];
    let x: Vec<_> = a.points.iter().map(project).collect();
    let y: Vec<_> = b.points.iter().map(project).collect();
    fn ordered(a: &Piece, b: &Piece, x: &[Q], y: &[Q], shared: bool) -> bool {
        let hi = x.iter().max().unwrap();
        let lo = y.iter().min().unwrap();
        if hi < lo {
            return !shared;
        }
        if !shared || hi != lo {
            return false;
        }
        let ae = [&a.points[0], a.points.last().unwrap()];
        let be = [&b.points[0], b.points.last().unwrap()];
        let common = ae
            .iter()
            .flat_map(|p| be.iter().map(move |q| (*p, *q)))
            .filter(|(p, q)| p == q)
            .count();
        if common == 0 || common > 2 {
            return false;
        }
        let flat_a = x.iter().all(|q| q == hi);
        let flat_b = y.iter().all(|q| q == lo);
        if flat_a && flat_b {
            return false;
        }
        // A flat span occupies the plane. The other span must have only its
        // common endpoint on it; a nonshared endpoint might otherwise hit it.
        if flat_a {
            for (p, q) in [(be[0], &y[0]), (be[1], y.last().unwrap())] {
                if !ae.contains(&p) && q == lo {
                    return false;
                }
            }
        }
        if flat_b {
            for (p, q) in [(ae[0], &x[0]), (ae[1], x.last().unwrap())] {
                if !be.contains(&p) && q == hi {
                    return false;
                }
            }
        }
        true
    }
    ordered(a, b, &x, &y, shared) || ordered(b, a, &y, &x, shared)
}
