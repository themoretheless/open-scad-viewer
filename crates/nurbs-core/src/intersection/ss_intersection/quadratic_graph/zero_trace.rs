//! Exact complete pcurve membership and signed components of a source conic.
//! These support-relative relations do not classify a trimmed body volume.
use super::*;
use crate::curve::Curve;
fn replay(owner: &QualifiedQuadraticGraphSS, max: usize) -> Result<Work> {
    let mut w = Work::new(max)?;
    for _ in 0..owner.operations() {
        w.tick()?;
    }
    owner.reverify()?;
    Ok(w)
}
fn product(a: &[f64], b: &[f64], w: &mut Work) -> Result<Vec<f64>> {
    check(
        a.len() + b.len() <= 18,
        "Exact graph trace polynomial exceeded degree16",
    )?;
    let mut out = vec![0.; a.len() + b.len() - 1];
    for (i, &a) in a.iter().enumerate() {
        for (j, &b) in b.iter().enumerate() {
            if a == 0. || b == 0. {
                continue;
            }
            let t = w.mul(a, b)?;
            out[i + j] = w.add(out[i + j], t)?;
        }
    }
    Ok(out)
}
impl QualifiedQuadraticGraphSS {
    /// Prove the complete unchanged clamped rational UV curve lies on the
    /// ORIGINAL graph difference zero set. Successful return is exact algebra
    /// work, not a sample residual or a Solid/topology permission. Both source
    /// replay and every coefficient operation share the supplied allowance.
    pub fn prove_zero_trace(
        &self,
        curve: &Curve,
        side: usize,
        max_operations: usize,
    ) -> Result<usize> {
        check(side < 2, "Choose an original graph support side")?;
        curve.validate()?;
        let n = curve.control_points.len();
        let domain = curve.domain();
        check(
            (1..=8).contains(&curve.degree)
                && !curve.periodic
                && n == curve.degree + 1
                && curve.control_points[0].len() == 2
                && curve.knots[..n].iter().all(|&t| t == domain[0])
                && curve.knots[n..].iter().all(|&t| t == domain[1]),
            "Exact zero trace requires one complete clamped rational UV Bezier curve",
        )?;
        let mut w = replay(self, max_operations)?;
        let den = rational_graph::power(&curve.weights, &mut w)?;
        let mut coords = Vec::new();
        for axis in 0..2 {
            let values = curve
                .control_points
                .iter()
                .zip(&curve.weights)
                .map(|(p, &weight)| w.mul(p[axis], weight))
                .collect::<Result<Vec<_>>>()?;
            let mut p = rational_graph::power(&values, &mut w)?;
            let d = self.data.domains[side][axis];
            let span = w.add(d[1], -d[0])?;
            for (value, &weight) in p.iter_mut().zip(&den) {
                let offset = w.mul(d[0], weight)?;
                let delta = w.add(*value, -offset)?;
                *value = w.div(delta, span)?;
            }
            coords.push(p);
        }
        let mut sum = vec![0.; 2 * curve.degree + 1];
        for i in 0..3 {
            for j in 0..3 {
                let c = w.add(self.data.heights[0][i][j], -self.data.heights[1][i][j])?;
                if c == 0. {
                    continue;
                }
                check(
                    i + j <= 2,
                    "Original zero trace needs a quadratic difference",
                )?;
                let mut term = vec![1.];
                for _ in 0..i {
                    term = product(&term, &coords[0], &mut w)?;
                }
                for _ in 0..j {
                    term = product(&term, &coords[1], &mut w)?;
                }
                for _ in i + j..2 {
                    term = product(&term, &den, &mut w)?;
                }
                for (out, &v) in sum.iter_mut().zip(&term) {
                    let v = w.mul(c, v)?;
                    *out = w.add(*out, v)?;
                }
            }
        }
        for value in sum {
            w.tick()?;
            check(
                value == 0.,
                "Original graph trace has a nonzero exact coefficient",
            )?;
        }
        Ok(w.used)
    }
    /// Signs of first-minus-second height over the entire outer and inner
    /// components of the unique contained ellipse. True means strictly above.
    /// The zero contour is excluded. A definite quadratic with a complete
    /// ellipse census has opposite signs on these connected components; the
    /// original constant coefficient fixes its sign without a sampled seed.
    pub fn prove_closed_height_sides(&self, max_operations: usize) -> Result<([bool; 2], usize)> {
        let mut w = replay(self, max_operations)?;
        check(
            self.kind() == QuadraticGraphSSKind::ClosedEllipse
                && self.branches().len() == 1
                && self.branches()[0].closed,
            "Height components require one complete original contained ellipse",
        )?;
        let corner = w.add(self.data.heights[0][0][0], -self.data.heights[1][0][0])?;
        check(
            corner != 0.,
            "Original exterior corner height sign must be strict",
        )?;
        Ok(([corner > 0., corner < 0.], w.used))
    }
}
#[cfg(test)]
mod tests;
