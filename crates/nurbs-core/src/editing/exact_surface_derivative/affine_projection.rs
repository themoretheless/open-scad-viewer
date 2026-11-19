//! Whole-grid affine projection proof in the original parameter clock.
use super::*;

/// Certify a pair of Cartesian coordinates by comparing the homogeneous
/// numerator of each coordinate with the global affine map times the weight
/// polynomial on every active knot cell. The reported work includes refusals.
pub fn rational_affine_coordinate_pair(
    surface: &Surface,
    max_work: u64,
) -> Result<(Option<[usize; 2]>, u64)> {
    surface.validate()?;
    check(
        max_work > 0 && max_work <= cad_predicates::MAX_WORK,
        "Invalid projection work limit",
    )?;
    let mut work = Work {
        used: 0,
        limit: max_work,
    };
    let pair = prove(surface, &mut work).ok().flatten();
    Ok((pair, work.used))
}

fn prove(s: &Surface, work: &mut Work) -> PResult<Option<[usize; 2]>> {
    let nu = s.control_points.len();
    let nv = s.control_points[0].len();
    if s.control_points[0][0].len() != 3 || s.periodic_u || s.periodic_v {
        return Ok(None);
    }
    for (k, d, n) in [(&s.knots_u, s.degree_u, nu), (&s.knots_v, s.degree_v, nv)] {
        if !k[..=d].iter().all(|t| *t == k[d]) || !k[n..].iter().all(|t| *t == k[n]) {
            return Ok(None);
        }
    }
    let u0 = work.lift(s.knots_u[s.degree_u])?;
    let v0 = work.lift(s.knots_v[s.degree_v])?;
    let du = work.lift(s.knots_u[nu])? - &u0;
    let dv = work.lift(s.knots_v[nv])? - &v0;
    let mut frames = Vec::new();
    for k in 0..3 {
        let origin = work.lift(s.control_points[0][0][k])?;
        let u = (work.lift(s.control_points[nu - 1][0][k])? - &origin) / &du;
        let v = (work.lift(s.control_points[0][nv - 1][k])? - &origin) / &dv;
        frames.push((origin, u, v));
    }
    let mut valid = [true; 3];
    for i in s.degree_u..nu {
        let u = [s.knots_u[i], s.knots_u[i + 1]];
        if u[0] == u[1] {
            continue;
        }
        for j in s.degree_v..nv {
            let v = [s.knots_v[j], s.knots_v[j + 1]];
            if v[0] == v[1] {
                continue;
            }
            let left = work.lift(u[0])?;
            let bottom = work.lift(v[0])?;
            let widths = [work.lift(u[1])? - &left, work.lift(v[1])? - &bottom];
            let h = extraction::cell(s, i, j, [u, v], &widths, work)?;
            for (k, (origin, a, b)) in frames.iter().enumerate() {
                if !valid[k] {
                    continue;
                }
                let offset = origin + a * (&left - &u0) + b * (&bottom - &v0);
                let affine = Poly::from([
                    ((0, 0), offset),
                    ((1, 0), a * &widths[0]),
                    ((0, 1), b * &widths[1]),
                ]);
                valid[k] = polynomial::multiply(&affine, &h[3], work)? == h[k];
            }
            if valid.iter().filter(|v| **v).count() < 2 {
                return Ok(None);
            }
        }
    }
    Ok([[0, 1], [0, 2], [1, 2]].into_iter().find(|&[i, j]| {
        valid[i] && valid[j] && &frames[i].1 * &frames[j].2 != &frames[j].1 * &frames[i].2
    }))
}
