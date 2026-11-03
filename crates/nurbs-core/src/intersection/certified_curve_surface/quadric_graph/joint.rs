//! Global projected disk certificate from an exact boundary and Jacobian.
use super::*;
mod bspline;
mod inverse;
mod poly;
use poly::{Poly, Work, LIMIT};
pub(super) struct Joint {
    patches: Vec<Patch>,
}
struct Patch {
    h: Vec<Vec<[Q; 3]>>,
    uv: [[Q; 2]; 2],
}
impl Joint {
    pub(super) fn prove(
        s: &Surface,
        axes: [usize; 2],
        o: &[Q; 2],
        u: &[Q; 2],
        v: &[Q; 2],
        det: &Q,
    ) -> Option<Self> {
        Self::prove_with_limit(s, axes, o, u, v, det, LIMIT)
    }
    fn prove_with_limit(
        s: &Surface,
        axes: [usize; 2],
        o: &[Q; 2],
        u: &[Q; 2],
        v: &[Q; 2],
        det: &Q,
        limit: u64,
    ) -> Option<Self> {
        let (p, r) = (s.degree_u, s.degree_v);
        if s.control_points.len() != p + 1 || s.control_points[0].len() != r + 1 {
            return bspline::prove(s, axes, o, u, v, det, limit);
        }
        let h: Vec<Vec<[Q; 3]>> = (0..=p)
            .map(|i| {
                (0..=r)
                    .map(|j| super::homogeneous_separable::pole(s, i, j, axes, o, u, v, det))
                    .collect()
            })
            .collect();
        if h.iter().flatten().any(|a| a[2] <= q(0.)) {
            return None;
        }
        // The ORIGINAL four edges map onto the four rectangle edges in
        // positive cyclic order. Full coefficient identities fix the normal
        // coordinate; exact scalar clock proofs establish edge homeomorphisms.
        if !(0..=r).all(|j| h[0][j][0] == q(0.) && h[p][j][0] == h[p][j][2])
            || !(0..=p).all(|i| h[i][0][1] == q(0.) && h[i][r][1] == h[i][r][2])
        {
            return None;
        }
        for boundary in [
            (0..=p)
                .map(|i| [h[i][0][0].clone(), h[i][0][2].clone()])
                .collect(),
            (0..=p)
                .map(|i| [h[i][r][0].clone(), h[i][r][2].clone()])
                .collect(),
            (0..=r)
                .map(|j| [h[0][j][1].clone(), h[0][j][2].clone()])
                .collect(),
            (0..=r)
                .map(|j| [h[p][j][1].clone(), h[p][j][2].clone()])
                .collect(),
        ] {
            super::clock::Clock::certified(boundary)?;
        }
        let mut work = Work::new(limit);
        if !positive_jacobian(&h, &mut work)? {
            return None;
        }
        // Boundary winding number is one. Every interior preimage is regular
        // with positive orientation, so its degree contribution is +1.
        // Therefore each interior rectangle point has exactly one preimage;
        // outside points have zero. An interior point cannot map onto the
        // boundary (local openness would give exterior preimages). Together
        // with the proved edge homeomorphisms this certifies the closed disk.
        Some(Self {
            patches: vec![Patch {
                h,
                uv: [[q(0.), q(1.)], [q(0.), q(1.)]],
            }],
        })
    }
    pub(super) fn enclosure(&self, r: &[[Q; 2]; 2]) -> Result<[[Q; 2]; 2]> {
        inverse::enclosure(&self.patches, r)
    }
}

fn positive_jacobian(h: &[Vec<[Q; 3]>], work: &mut Work) -> Option<bool> {
    let channel = |k: usize| {
        Poly(
            h.iter()
                .map(|row| row.iter().map(|a| a[k].clone()).collect())
                .collect(),
        )
    };
    let (x, y, w) = (channel(0), channel(1), channel(2));
    let (xu, xv, yu, yv, wu, wv) = (
        x.derivative(0, work)?,
        x.derivative(1, work)?,
        y.derivative(0, work)?,
        y.derivative(1, work)?,
        w.derivative(0, work)?,
        w.derivative(1, work)?,
    );
    // det d(X/W,Y/W) = K/W^3. K is the homogeneous 3x3
    // determinant, assembled exactly in degree (3p-1,3r-1).
    let a = yv.mul(&w, work)?.sub(&y.mul(&wv, work)?, work)?;
    let b = yu.mul(&w, work)?.sub(&y.mul(&wu, work)?, work)?;
    let c = yu.mul(&wv, work)?.sub(&yv.mul(&wu, work)?, work)?;
    let k = xu
        .mul(&a, work)?
        .sub(&xv.mul(&b, work)?, work)?
        .add(&x.mul(&c, work)?, work)?;
    k.positive(work, 0)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn proof_budget_never_becomes_a_chart_certificate() {
        let f: Vec<value_codec::Value> = value_codec::from_str(include_str!(
            "../../../../../../tools/browser-qualification/joint-quadric-graph-fixtures.json"
        ))
        .unwrap();
        let s: Surface = value_codec::from_value(f[0]["query"]["patch"].clone()).unwrap();
        let l = f[0]["scale"].as_f64().unwrap();
        let (o, u, v) = ([q(0.), q(0.)], [q(l), q(0.)], [q(0.), q(l)]);
        let det = q(l) * q(l);
        assert!(Joint::prove_with_limit(&s, [0, 1], &o, &u, &v, &det, 0).is_none());
        assert!(Joint::prove_with_limit(&s, [0, 1], &o, &u, &v, &det, LIMIT).is_some());
    }
}
