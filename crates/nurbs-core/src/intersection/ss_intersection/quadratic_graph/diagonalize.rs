//! Exact completion of squares. The shear is a source-coordinate identity,
//! not a rounded eigenvector fit or sampled recognition of a conic.
use super::*;
pub(super) fn form(f: &Poly, w: &mut Work) -> Result<([f64; 3], [f64; 2], [[f64; 2]; 2])> {
    let a = f[2][0];
    if f[1][1] == 0. {
        let b = f[0][2];
        let mut center = [0.5; 2];
        for (axis, (quadratic, linear)) in [(a, f[1][0]), (b, f[0][1])].into_iter().enumerate() {
            if quadratic == 0. {
                check(
                    linear == 0.,
                    "Linear graph difference uses the regular branch path",
                )?;
            } else {
                let den = w.mul(2., quadratic)?;
                center[axis] = w.div(-linear, den)?;
            }
        }
        let mut k = f[0][0];
        for (c, coefficient) in [(center[0], a), (center[1], b)] {
            let square = w.mul(c, c)?;
            let term = w.mul(coefficient, square)?;
            k = w.add(k, -term)?;
        }
        return Ok(([a, b, k], center, [[1., 0.], [0., 1.]]));
    }
    if a == 0. {
        if f[0][2] != 0. {
            let mut transpose = [[0.; 3]; 3];
            for i in 0..3 {
                for j in 0..3 {
                    transpose[i][j] = f[j][i];
                }
            }
            let (diagonal, center, basis) = form(&transpose, w)?;
            return Ok((diagonal, [center[1], center[0]], [basis[1], basis[0]]));
        }
        // uv = (p+q, p-q), so C*u*v = C*p²-C*q².
        let a = f[1][1];
        let b = -a;
        let lp = w.add(f[1][0], f[0][1])?;
        let lq = w.add(f[1][0], -f[0][1])?;
        let twice_a = w.mul(2., a)?;
        let twice_b = w.mul(2., b)?;
        let pc = w.div(-lp, twice_a)?;
        let qc = w.div(-lq, twice_b)?;
        let center = [w.add(pc, qc)?, w.add(pc, -qc)?];
        let mut k = f[0][0];
        for (c, coefficient) in [(pc, a), (qc, b)] {
            let square = w.mul(c, c)?;
            let term = w.mul(coefficient, square)?;
            k = w.add(k, -term)?;
        }
        return Ok(([a, b, k], center, [[1., 1.], [1., -1.]]));
    }
    let twice_a = w.mul(2., a)?;
    let h = w.div(f[1][1], twice_a)?;
    let hh = w.mul(h, h)?;
    let correction = w.mul(a, hh)?;
    let b = w.add(f[0][2], -correction)?;
    let linear_correction = w.mul(f[1][0], h)?;
    let linear_q = w.add(f[0][1], -linear_correction)?;
    let pc = w.div(-f[1][0], twice_a)?;
    let qc = if b == 0. {
        check(
            linear_q == 0.,
            "Mixed parabolic stratum needs a separate exact classifier",
        )?;
        0.5
    } else {
        let twice_b = w.mul(2., b)?;
        w.div(-linear_q, twice_b)?
    };
    let shift = w.mul(h, qc)?;
    let center = [w.add(pc, -shift)?, qc];
    let mut k = f[0][0];
    for (c, coefficient) in [(pc, a), (qc, b)] {
        let square = w.mul(c, c)?;
        let term = w.mul(coefficient, square)?;
        k = w.add(k, -term)?;
    }
    Ok(([a, b, k], center, [[1., -h], [0., 1.]]))
}
/// A*cos(t)+B*sin(t) has exact range +/-sqrt(A²+B²). Apply the
/// original-coordinate basis BEFORE testing rectangle coverage. Separate
/// transformed radii alone would wrongly admit a clipped tilted ellipse.
pub(super) fn contained(d: &Data, w: &mut Work) -> Result<()> {
    let squares = [
        w.id(I::point(-d.diagonal[2]), I::point(d.diagonal[0]))?,
        w.id(I::point(-d.diagonal[2]), I::point(d.diagonal[1]))?,
    ];
    for axis in 0..2 {
        let mut amplitude_squared = I::point(0.);
        for j in 0..2 {
            let b = I::point(d.basis[axis][j]);
            let bb = w.im(b, b)?;
            let term = w.im(bb, squares[j])?;
            amplitude_squared = w.ia(amplitude_squared, term)?;
        }
        let amplitude = w.sqrt(amplitude_squared)?;
        let low = w.is(I::point(d.center[axis]), amplitude)?;
        let high = w.ia(I::point(d.center[axis]), amplitude)?;
        check(
            low.lo > 0. && high.hi < 1.,
            "Closed ellipse must be strictly contained in both original graph patches",
        )?;
    }
    Ok(())
}
