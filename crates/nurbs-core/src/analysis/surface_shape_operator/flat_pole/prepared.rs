use super::*;
pub(super) struct Prepared {
    pub net: Vec<Vec<[I; 4]>>,
    pub pole: [f64; 3],
    pub axes: [usize; 3],
    pub source_u: [f64; 2],
}
fn elevate(mut p: Vec<I>, degree: usize) -> Result<Vec<I>> {
    while p.len() < degree + 1 {
        let old = p;
        let n = old.len();
        let mut out = vec![I::point(0.); n + 1];
        out[0] = old[0];
        out[n] = old[n - 1];
        for i in 1..n {
            let a = I::point(i as f64).div(I::point(n as f64))?;
            out[i] = old[i - 1].mul(a)?.add(old[i].mul(I::point(1.).sub(a)?)?)?;
        }
        p = out;
    }
    Ok(p)
}
impl Prepared {
    pub fn new(s: &Surface, end: bool, height: usize) -> Result<Option<Self>> {
        let (p, q, n, m) = (
            s.degree_u,
            s.degree_v,
            s.control_points.len(),
            s.control_points[0].len(),
        );
        check(
            (3..=16).contains(&p)
                && (1..=16).contains(&q)
                && height < 3
                && !s.periodic_u
                && !s.periodic_v,
            "Flat pole needs original nonperiodic degrees 3..16 by 1..16",
        )?;
        check(
            m == q + 1
                && s.knots_v[..=q].iter().all(|&x| x == s.knots_v[q])
                && s.knots_v[q + 1..].iter().all(|&x| x == s.knots_v[q + 1]),
            "Flat pole needs one original V span",
        )?;
        check(
            s.knots_u[..=p].iter().all(|&x| x == s.knots_u[p])
                && s.knots_u[n..].iter().all(|&x| x == s.knots_u[n]),
            "Flat pole needs clamped original U endpoints",
        )?;
        let span = if end { n - 1 } else { p };
        let source_u = [s.knots_u[span], s.knots_u[span + 1]];
        check(
            source_u[0] < source_u[1],
            "Flat pole lost its original nonzero span",
        )?;
        let inner = source_u[usize::from(!end)];
        if s.knots_u[p] < inner && inner < s.knots_u[n] {
            check(
                s.knots_u.iter().filter(|&&x| x == inner).count() == p,
                "Pole span must retain its original Bernstein controls",
            )?;
        }
        let row = |i| if end { n - 1 - i } else { i };
        let pole: [f64; 3] = std::array::from_fn(|k| s.control_points[row(0)][0][k]);
        if !s.control_points[row(0)]
            .iter()
            .all(|x| x.as_slice() == pole.as_slice())
        {
            return Ok(None);
        }
        if !(0..3).all(|i| {
            s.control_points[row(i)]
                .iter()
                .all(|x| x[height] == pole[height])
        }) {
            return Ok(None);
        }
        let axes = [(height + 1) % 3, (height + 2) % 3, height];
        let mut net = vec![vec![[I::point(0.); 4]; q + 1]; p + 1];
        for v in 0..=q {
            for i in 0..=p {
                net[i][v][3] = I::point(s.weights[row(i)][v]);
            }
            for k in 0..3 {
                let order = if k == 2 { 3 } else { 1 };
                let mut factor = Vec::new();
                for i in order..=p {
                    let mut ratio = I::point(1.);
                    for j in 0..order {
                        ratio =
                            ratio.mul(I::point((p - j) as f64).div(I::point((i - j) as f64))?)?;
                    }
                    let value = I::point(s.control_points[row(i)][v][axes[k]])
                        .sub(I::point(pole[axes[k]]))?
                        .mul(I::point(s.weights[row(i)][v]))?
                        .mul(ratio)?;
                    factor.push(value);
                }
                let coeff = elevate(factor, p)?;
                for i in 0..=p {
                    net[i][v][k] = coeff[i];
                }
            }
        }
        Ok(Some(Self {
            net,
            pole,
            axes,
            source_u,
        }))
    }
}
