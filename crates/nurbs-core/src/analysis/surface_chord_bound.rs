//! Certified flatness bounds for parameter cells of a NURBS surface.
//!
//! For a parameter triangle with vertices `p_i` inside the rectangle `D` and a
//! point `x` with barycentric weights `λ_i`, Taylor's formula with integral
//! remainder gives `S(x) - Σ λ_i S(p_i) = -Σ λ_i ∫₀¹ (1-t) D²S(..)[d_i, d_i] dt`
//! with `d_i = p_i - x`. In the unit coordinates of `D` every `|d_i|` is at
//! most one per axis, so `|S(x) - Σ λ_i S(p_i)| <= (|S_uu| + 2|S_uv| + |S_vv|)/2`
//! with suprema over `D`. The formula needs `S ∈ C¹` with piecewise bounded
//! second derivatives, so `D` may cross interior knots only where the
//! multiplicity leaves at least `C¹` continuity; the suprema are taken per
//! knot piece from interval jets of the original rational surface and
//! rescaled to the unit coordinates of `D`.
use super::surface_measure::jets;
use crate::{
    Error, Result, check,
    distance_bounds::{Interval as I, box_distance},
    numerics::interval_vec3::norm,
    surface::Surface,
};

/// Knot pieces of `[a, b]`, refusing interior knots below `C¹`.
fn pieces(knots: &[f64], degree: usize, count: usize, [a, b]: [f64; 2]) -> Result<Vec<(usize, [f64; 2])>> {
    check(a < b && a >= knots[degree] && b <= knots[count], "Chord cell must lie inside the surface domain")?;
    for &k in &knots[degree + 1..count] {
        if a < k && k < b {
            let multiplicity = knots.iter().filter(|&&x| x == k).count();
            if multiplicity + 1 > degree {
                return Err(Error::new(
                    "NURBS_CHORD_CELL_CROSSES_KINK",
                    "Chord cell crosses an interior knot without C1 continuity",
                ));
            }
        }
    }
    Ok((degree..count)
        .filter(|&k| knots[k] < knots[k + 1])
        .filter_map(|k| {
            let lo = a.max(knots[k]);
            let hi = b.min(knots[k + 1]);
            (lo < hi).then_some((k, [lo, hi]))
        })
        .collect())
}

/// Upper bound on `|S(x) - Σ λ_i S(p_i)|` for every parameter triangle whose
/// vertices lie in `domain`, including triangles degenerate in 3D.
pub fn cell_flatness(s: &Surface, domain: [[f64; 2]; 2]) -> Result<f64> {
    let pu = pieces(&s.knots_u, s.degree_u, s.control_points.len(), domain[0])?;
    let pv = pieces(&s.knots_v, s.degree_v, s.control_points[0].len(), domain[1])?;
    let width = [domain[0][1] - domain[0][0], domain[1][1] - domain[1][0]];
    let (mut uu, mut uv, mut vv) = (0f64, 0f64, 0f64);
    for &(ku, du) in &pu {
        for &(kv, dv) in &pv {
            let j = jets::calculate(s, [ku, kv], [du, dv], None)?;
            // Jets are derivatives in the unit coordinates of the piece.
            let su = I::point(width[0]).div(I::point(du[1] - du[0]))?;
            let sv = I::point(width[1]).div(I::point(dv[1] - dv[0]))?;
            let m = |v: [I; 3], f: I| -> Result<f64> { Ok(norm(v)?.mul(f)?.hi) };
            uu = uu.max(m(j[2][0], su.mul(su)?)?);
            uv = uv.max(m(j[1][1], su.mul(sv)?)?);
            vv = vv.max(m(j[0][2], sv.mul(sv)?)?);
        }
    }
    let bound = ((uu + 2. * uv + vv) / 2.).next_up().next_up();
    check(bound.is_finite(), "Chord cell bound is not finite")?;
    Ok(bound)
}

/// Enclose the interpolation error of every parameter triangle in `domain`.
///
/// In unit rectangle coordinates the weighted variance of each coordinate
/// of the triangle vertices is at most 1/4. Cauchy–Schwarz bounds the
/// weighted absolute mixed product by 1/4 as well. Applying these bounds
/// to the integral Taylor remainder gives (M_uu + 2 M_uv + M_vv)/8,
/// where each M is an upper bound on the corresponding second jet norm.
/// The reference vertices are exact surface points in the same chart.
/// A cell crossing a knot must have at least C¹ continuity in that axis.
pub fn cell_interpolation_error(
    s: &Surface,
    domain: [[f64; 2]; 2],
    max_cells: usize,
) -> Result<f64> {
    s.validate()?;
    check((1..=4096).contains(&max_cells), "Chord cell budget must be 1..4096")?;
    let u = pieces(&s.knots_u, s.degree_u, s.control_points.len(), domain[0])?;
    let v = pieces(&s.knots_v, s.degree_v, s.control_points[0].len(), domain[1])?;
    if u.len() * v.len() > max_cells {
        return Err(crate::resource("Chord knot rectangles exceed the work budget"));
    }
    let width = |d: [f64; 2]| I::point(d[1]).sub(I::point(d[0]));
    let [u_width, v_width] = [width(domain[0])?, width(domain[1])?];
    let mut maxima = [0f64; 3];
    for &(ku, du) in &u {
        for &(kv, dv) in &v {
            let j = jets::calculate(s, [ku, kv], [du, dv], None)?;
            let su = u_width.div(width(du)?)?;
            let sv = v_width.div(width(dv)?)?;
            for (maximum, (jet, scale)) in maxima.iter_mut().zip([
                (j[2][0], su.mul(su)?),
                (j[1][1], su.mul(sv)?),
                (j[0][2], sv.mul(sv)?),
            ]) {
                *maximum = maximum.max(norm(jet)?.mul(scale)?.hi);
            }
        }
    }
    let [uu, uv, vv] = maxima.map(I::point);
    Ok(uu.add(uv.mul(I::point(2.))?)?.add(vv)?.mul(I::point(0.125))?.hi)
}

/// Upper bound on `|S(uv) - q|`, evaluated with interval jets.
pub fn point_distance(s: &Surface, uv: [f64; 2], q: [f64; 3]) -> Result<f64> {
    let locate = |knots: &[f64], degree: usize, count: usize, t: f64| -> Result<(usize, [f64; 2], usize)> {
        check(t >= knots[degree] && t <= knots[count], "Point lies outside the surface domain")?;
        let k = (degree..count)
            .filter(|&k| knots[k] < knots[k + 1] && knots[k] <= t && t <= knots[k + 1])
            .last()
            .ok_or_else(|| Error::new("NURBS_INPUT_INVALID", "Surface has no active knot span"))?;
        Ok(if t > knots[k] { (k, [knots[k], t], 1) } else { (k, [t, knots[k + 1]], 0) })
    };
    let (ku, du, cu) = locate(&s.knots_u, s.degree_u, s.control_points.len(), uv[0])?;
    let (kv, dv, cv) = locate(&s.knots_v, s.degree_v, s.control_points[0].len(), uv[1])?;
    let j = jets::calculate(s, [ku, kv], [du, dv], Some([cu, cv]))?;
    let origin = &s.control_points[ku - s.degree_u][kv - s.degree_v];
    let exact: Vec<I> = (0..3).map(|a| j[0][0][a].add(I::point(origin[a]))).collect::<Result<_>>()?;
    let vertex: Vec<I> = q.iter().map(|&x| I::point(x)).collect();
    let d = box_distance(&exact, &vertex)?.1;
    check(d.is_finite(), "Point distance bound is not finite")?;
    Ok(d)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rational_circle_cylinder_cells_bound_sampled_chords() {
        let s = crate::primitives::cylinder([0.; 3], 2., 3.).unwrap();
        let [u0, u1] = [s.knots_u[s.degree_u], s.knots_u[s.control_points.len()]];
        let [v0, v1] = [s.knots_v[s.degree_v], s.knots_v[s.control_points[0].len()]];
        let n = 16;
        for i in 0..n {
            let a = u0 + (u1 - u0) * i as f64 / n as f64;
            let b = u0 + (u1 - u0) * (i + 1) as f64 / n as f64;
            let Ok(bound) = cell_flatness(&s, [[a, b], [v0, v1]]) else { continue };
            let pa = s.evaluate(a, v0).unwrap().point;
            let pb = s.evaluate(b, v0).unwrap().point;
            for k in 0..=20 {
                let t = k as f64 / 20.;
                let p = s.evaluate(a + (b - a) * t, v0).unwrap().point;
                let chord: Vec<f64> = (0..3).map(|c| pa[c] + (pb[c] - pa[c]) * t).collect();
                let d = (0..3).map(|c| (p[c] - chord[c]).powi(2)).sum::<f64>().sqrt();
                assert!(d <= bound, "{d} > {bound}");
            }
            assert!(point_distance(&s, [a, v0], [pa[0], pa[1], pa[2]]).unwrap() < 1e-12);
        }
    }
}
