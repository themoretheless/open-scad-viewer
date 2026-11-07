use super::*;
#[derive(Clone, Copy)]
struct P {
    a: I,
    b: I,
    d: I,
}
impl P {
    fn new(a: I, b: I, d: I) -> Result<Self> {
        let d = I::new(d.lo.max(b.sub(a)?.lo), d.hi.min(b.sub(a)?.hi))?;
        let b = I::new(b.lo.max(a.add(d)?.lo), b.hi.min(a.add(d)?.hi))?;
        Ok(Self { a, b, d })
    }
    fn zero() -> Self {
        Self {
            a: I::point(0.),
            b: I::point(0.),
            d: I::point(0.),
        }
    }
    fn add(self, x: Self) -> Result<Self> {
        Self::new(self.a.add(x.a)?, self.b.add(x.b)?, self.d.add(x.d)?)
    }
    fn sub(self, x: Self) -> Result<Self> {
        Self::new(self.a.sub(x.a)?, self.b.sub(x.b)?, self.d.sub(x.d)?)
    }
    fn mul(self, x: Self) -> Result<Self> {
        Self::new(
            self.a.mul(x.a)?,
            self.b.mul(x.b)?,
            self.a
                .mul(x.d)?
                .add(self.d.mul(x.a)?)?
                .add(self.d.mul(x.d)?)?,
        )
    }
    fn div(self, x: Self) -> Result<Self> {
        Self::new(
            self.a.div(x.a)?,
            self.b.div(x.b)?,
            self.d.mul(x.a)?.sub(self.a.mul(x.d)?)?.div(x.a.mul(x.b)?)?,
        )
    }
    fn neg(self) -> Result<Self> {
        Self::zero().sub(self)
    }
}
fn dot(a: [P; 3], b: [P; 3]) -> Result<P> {
    (0..3).try_fold(P::zero(), |s, k| s.add(a[k].mul(b[k])?))
}
pub(super) fn bounds(
    a: &jets::Jets,
    b: &jets::Jets,
    delta: [[I; 3]; 3],
) -> Result<Option<[[f64; 2]; 2]>> {
    bounds_full(
        a,
        b,
        [
            delta[0],
            delta[1],
            delta[2],
            [I::point(0.); 3],
            [I::point(0.); 3],
        ],
    )
}
/// Exact rank-one shape operator for a curve extruded along a literal unit
/// coordinate axis. Callers must establish the zero axial profile and unit
/// transverse jet; otherwise this specialization refuses its contract.
pub(super) fn extrusion_bounds(
    a: &jets::Jets,
    b: &jets::Jets,
    delta: [[I; 3]; 3],
    axis: usize,
) -> Result<Option<[[f64; 2]; 2]>> {
    let point = |x: I, v: f64| x.lo == v && x.hi == v;
    check(
        axis < 3
            && [a, b].iter().all(|j| {
                (0..3).all(|k| {
                    point(j[0][1][k], if k == axis { 1. } else { 0. })
                        && point(j[1][1][k], 0.)
                        && point(j[0][2][k], 0.)
                }) && point(j[1][0][axis], 0.)
                    && point(j[2][0][axis], 0.)
            }),
        "Rank-one extrusion bounds need literal product jets",
    )?;
    let field = |order: usize| -> Result<[P; 3]> {
        let mut p = [P::zero(); 3];
        for k in 0..3 {
            p[k] = P::new(a[order][0][k], b[order][0][k], delta[order - 1][k])?;
        }
        Ok(p)
    };
    let u = field(1)?;
    let w = field(2)?;
    let mut cross = [P::zero(); 3];
    for k in 0..3 {
        let i = (k + 1) % 3;
        let j = (k + 2) % 3;
        cross[k] = u[i]
            .mul(P::new(
                I::point(if j == axis { 1. } else { 0. }),
                I::point(if j == axis { 1. } else { 0. }),
                I::point(0.),
            )?)?
            .sub(u[j].mul(P::new(
                I::point(if i == axis { 1. } else { 0. }),
                I::point(if i == axis { 1. } else { 0. }),
                I::point(0.),
            )?)?)?;
    }
    let aa = box_distance(&cross.map(|x| x.a), &[I::point(0.); 3])?;
    let bb = box_distance(&cross.map(|x| x.b), &[I::point(0.); 3])?;
    if aa.0 <= 0. || bb.0 <= 0. {
        return Ok(None);
    }
    let aa = I::new(aa.0, aa.1)?;
    let bb = I::new(bb.0, bb.1)?;
    let speed = P::new(aa, bb, dot(cross, cross)?.d.div(aa.add(bb)?)?)?;
    let curvature = dot(cross, w)?.div(speed.mul(speed)?.mul(speed)?)?;
    let mut normal = [P::zero(); 3];
    let mut tangent = normal;
    for k in 0..3 {
        normal[k] = cross[k].div(speed)?;
        tangent[k] = u[k].div(speed)?;
    }
    let mut tensor = [I::point(0.); 9];
    for i in 0..3 {
        for j in 0..3 {
            tensor[3 * i + j] = curvature.mul(tangent[i])?.mul(tangent[j])?.d;
        }
    }
    let nd = box_distance(&normal.map(|x| x.d), &[I::point(0.); 3])?;
    let td = box_distance(&tensor, &[I::point(0.); 9])?;
    Ok(Some([[nd.0, nd.1.min(2.)], [td.0, td.1]]))
}
#[cfg(test)]
mod rank_one_tests {
    use super::*;
    fn parabola(speed: f64, coefficient: f64) -> jets::Jets {
        let mut j = [[[I::point(0.); 3]; 4]; 4];
        j[1][0][0] = I::point(speed);
        j[2][0][2] = I::point(2. * coefficient * speed * speed);
        j[0][1][1] = I::point(1.);
        j
    }
    #[test]
    fn rank_one_operator_matches_analytic_curvature_and_positive_reclocking() {
        let a = parabola(1., 1.);
        let b = parabola(2., 1.);
        let mut d = [[I::point(0.); 3]; 3];
        d[0][0] = I::point(1.);
        d[1][2] = I::point(6.);
        let equal = extrusion_bounds(&a, &b, d, 1).unwrap().unwrap();
        assert!(equal[0][1] < 1e-10 && equal[1][1] < 1e-10, "{equal:?}");
        let b = parabola(2., 1.1);
        d[1][2] = b[2][0][2].sub(a[2][0][2]).unwrap();
        let different = extrusion_bounds(&a, &b, d, 1).unwrap().unwrap();
        assert!(
            different[1][0] > 0.199999 && different[1][1] < 0.200001,
            "{different:?}"
        );
    }
    #[test]
    fn a_nonproduct_jet_cannot_use_the_rank_one_shortcut() {
        let a = parabola(1., 1.);
        let mut b = a;
        b[1][0][1] = I::point(f64::from_bits(1));
        assert!(extrusion_bounds(&a, &b, [[I::point(0.); 3]; 3], 1).is_err());
    }
}
pub(super) fn bounds_full(
    a: &jets::Jets,
    b: &jets::Jets,
    delta: [[I; 3]; 5],
) -> Result<Option<[[f64; 2]; 2]>> {
    let field = |u: usize, v: usize| -> Result<[P; 3]> {
        let d = match (u, v) {
            (1, 0) => delta[0],
            (2, 0) => delta[1],
            (1, 1) => delta[2],
            (0, 1) => delta[3],
            (0, 2) => delta[4],
            _ => [I::point(0.); 3],
        };
        let mut out = [P::zero(); 3];
        for k in 0..3 {
            out[k] = P::new(a[u][v][k], b[u][v][k], d[k])?;
        }
        Ok(out)
    };
    let (u, v) = (field(1, 0)?, field(0, 1)?);
    let mut n = [P::zero(); 3];
    for k in 0..3 {
        let (i, j) = ((k + 1) % 3, (k + 2) % 3);
        n[k] = u[i].mul(v[j])?.sub(u[j].mul(v[i])?)?;
    }
    let aa = box_distance(&n.map(|x| x.a), &[I::point(0.); 3])?;
    let bb = box_distance(&n.map(|x| x.b), &[I::point(0.); 3])?;
    if aa.0 <= 0. || bb.0 <= 0. {
        return Ok(None);
    }
    let (aa, bb) = (I::new(aa.0, aa.1)?, I::new(bb.0, bb.1)?);
    let area = P::new(aa, bb, dot(n, n)?.d.div(aa.add(bb)?)?)?;
    for x in &mut n {
        *x = x.div(area)?;
    }
    let determinant = area.mul(area)?;
    let (e, f, g) = (dot(u, u)?, dot(u, v)?, dot(v, v)?);
    let inverse = [
        [g.div(determinant)?, f.neg()?.div(determinant)?],
        [f.neg()?.div(determinant)?, e.div(determinant)?],
    ];
    let second = [
        [dot(n, field(2, 0)?)?, dot(n, field(1, 1)?)?],
        [dot(n, field(1, 1)?)?, dot(n, field(0, 2)?)?],
    ];
    let mut coefficients = [[P::zero(); 2]; 2];
    for a in 0..2 {
        for b in 0..2 {
            for r in 0..2 {
                for s in 0..2 {
                    coefficients[a][b] = coefficients[a][b]
                        .add(inverse[a][r].mul(second[r][s])?.mul(inverse[s][b])?)?;
                }
            }
        }
    }
    let vectors = [u, v];
    let mut tensor = [I::point(0.); 9];
    for a in 0..3 {
        for b in 0..3 {
            let mut value = P::zero();
            for r in 0..2 {
                for s in 0..2 {
                    value =
                        value.add(vectors[r][a].mul(coefficients[r][s])?.mul(vectors[s][b])?)?;
                }
            }
            tensor[3 * a + b] = value.d;
        }
    }
    let nd = box_distance(&n.map(|x| x.d), &[I::point(0.); 3])?;
    let td = box_distance(&tensor, &[I::point(0.); 9])?;
    Ok(Some([[nd.0, nd.1.min(2.)], [td.0, td.1]]))
}
