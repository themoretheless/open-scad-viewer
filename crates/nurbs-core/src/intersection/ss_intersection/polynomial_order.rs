//! Exact polynomial material separators. No branch fitting or sampled sign.
use super::*;
use num_rational::BigRational as Q;
use num_traits::{One, Signed, Zero};
use std::collections::BTreeMap;
type P = BTreeMap<[usize; 2], Q>;
type Net = Vec<Vec<Q>>;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct GraphOrderLimits {
    pub max_cells: usize,
    pub max_depth: usize,
}
impl Default for GraphOrderLimits {
    fn default() -> Self {
        Self {
            max_cells: 4096,
            max_depth: 24,
        }
    }
}
#[derive(Clone, Debug)]
pub struct QualifiedGraphMaterialOrder {
    source: FiberSources,
    limits: GraphOrderLimits,
    cells: usize,
    square: bool,
}
fn q(v: f64) -> Result<Q> {
    Q::from_float(v).ok_or_else(|| crate::input("Nonfinite polynomial separator input"))
}
fn c(n: usize, k: usize) -> usize {
    if k > n {
        0
    } else {
        (0..k).fold(1, |v, i| v * (n - i) / (i + 1))
    }
}
fn admit(s: &Surface) -> Result<()> {
    s.validate()?;
    check(
        !s.periodic_u
            && !s.periodic_v
            && (1..=8).contains(&s.degree_u)
            && (1..=8).contains(&s.degree_v)
            && s.control_points.len() == s.degree_u + 1
            && s.control_points[0].len() == s.degree_v + 1
            && s.control_points.iter().flatten().all(|v| v.len() == 3),
        "Separator requires one degree1..8 spatial polynomial span",
    )?;
    for (d, k) in [(s.degree_u, &s.knots_u), (s.degree_v, &s.knots_v)] {
        check(
            k.len() == 2 * (d + 1)
                && k[..=d].iter().all(|v| *v == k[d])
                && k[d + 1..].iter().all(|v| *v == k[d + 1]),
            "Separator requires full clamped original spans",
        )?;
    }
    let w = s.weights[0][0];
    check(
        w > 0. && s.weights.iter().flatten().all(|&v| v == w),
        "Separator requires positive constant weights",
    )
}
fn add(p: &mut P, e: [usize; 2], a: Q) {
    let v = p.entry(e).or_default();
    *v += a;
    if v.is_zero() {
        p.remove(&e);
    }
}
fn mul(a: &P, b: &P) -> Result<P> {
    let mut p = P::new();
    for (e, a) in a {
        for (f, b) in b {
            let k = [e[0] + f[0], e[1] + f[1]];
            check(
                k[0] + k[1] <= 16,
                "Separator polynomial degree budget exceeded",
            )?;
            add(&mut p, k, a * b);
        }
    }
    Ok(p)
}
fn bernstein_power(s: &Surface, axis: usize) -> Result<P> {
    let (du, dv) = (s.degree_u, s.degree_v);
    let mut out = P::new();
    for i in 0..=du {
        for j in 0..=dv {
            let v = q(s.control_points[i][j][axis])?;
            if v.is_zero() {
                continue;
            }
            for k in i..=du {
                for l in j..=dv {
                    let factor = Q::from_integer(
                        (c(du, i) * c(du - i, k - i) * c(dv, j) * c(dv - j, l - j)).into(),
                    );
                    let value = &v * factor;
                    if (k - i + l - j) % 2 == 0 {
                        add(&mut out, [k, l], value);
                    } else {
                        add(&mut out, [k, l], -value);
                    }
                }
            }
        }
    }
    Ok(out)
}
fn affine_xy(s: &Surface) -> Result<[[Q; 3]; 2]> {
    admit(s)?;
    let xy = std::array::from_fn(|k| {
        let a = q(s.control_points[0][0][k]).unwrap();
        [
            a.clone(),
            q(s.control_points[s.degree_u][0][k]).unwrap() - &a,
            q(s.control_points[0][s.degree_v][k]).unwrap() - &a,
        ]
    });
    for i in 0..=s.degree_u {
        for j in 0..=s.degree_v {
            for k in 0..2 {
                check(
                    q(s.control_points[i][j][k])?
                        == &xy[k][0]
                            + &xy[k][1] * Q::new(i.into(), s.degree_u.into())
                            + &xy[k][2] * Q::new(j.into(), s.degree_v.into()),
                    "Original whole-face affine XY identity failed",
                )?;
            }
        }
    }
    Ok(xy)
}
fn inverse_xy(s: &Surface) -> Result<[[Q; 3]; 2]> {
    let a = affine_xy(s)?;
    let d = &a[0][1] * &a[1][2] - &a[0][2] * &a[1][1];
    check(!d.is_zero(), "Graph barrier XY projection is singular")?;
    let u = [
        (-&a[1][2] * &a[0][0] + &a[0][2] * &a[1][0]) / &d,
        &a[1][2] / &d,
        -&a[0][2] / &d,
    ];
    let v = [
        (&a[1][1] * &a[0][0] - &a[0][1] * &a[1][0]) / &d,
        -&a[1][1] / &d,
        &a[0][1] / &d,
    ];
    Ok([u, v])
}
fn map_to(barrier: &Surface, face: &Surface) -> Result<[[Q; 3]; 2]> {
    let inv = inverse_xy(barrier)?;
    let a = affine_xy(face)?;
    Ok(std::array::from_fn(|k| {
        [
            &inv[k][0] + &inv[k][1] * &a[0][0] + &inv[k][2] * &a[1][0],
            &inv[k][1] * &a[0][1] + &inv[k][2] * &a[1][1],
            &inv[k][1] * &a[0][2] + &inv[k][2] * &a[1][2],
        ]
    }))
}
fn affine_p(a: &[Q; 3]) -> P {
    let mut p = P::new();
    for (e, c) in [[0, 0], [1, 0], [0, 1]].into_iter().zip(a) {
        add(&mut p, e, c.clone());
    }
    p
}
fn composed_height(barrier: &Surface, face: &Surface) -> Result<P> {
    let map = map_to(barrier, face)?;
    let mut powers = vec![];
    for (k, degree) in [barrier.degree_u, barrier.degree_v].into_iter().enumerate() {
        let a = affine_p(&map[k]);
        let mut p = vec![P::from([([0, 0], Q::one())])];
        for i in 0..degree {
            p.push(mul(&p[i], &a)?);
        }
        powers.push(p);
    }
    let mut out = P::new();
    for (e, v) in bernstein_power(barrier, 2)? {
        let term = mul(&powers[0][e[0]], &powers[1][e[1]])?;
        for (k, x) in term {
            add(&mut out, k, &v * x);
        }
    }
    Ok(out)
}
fn gap(first: &Surface, second: &Surface) -> Result<P> {
    let mut p = composed_height(second, first)?;
    for (e, v) in bernstein_power(first, 2)? {
        add(&mut p, e, -v);
    }
    Ok(p)
}
fn to_net(p: &P) -> Net {
    let du = p.keys().map(|e| e[0]).max().unwrap_or(0);
    let dv = p.keys().map(|e| e[1]).max().unwrap_or(0);
    let mut n = vec![vec![Q::zero(); dv + 1]; du + 1];
    for i in 0..=du {
        for j in 0..=dv {
            for (e, v) in p {
                if e[0] <= i && e[1] <= j {
                    n[i][j] += v
                        * Q::new(c(i, e[0]).into(), c(du, e[0]).into())
                        * Q::new(c(j, e[1]).into(), c(dv, e[1]).into());
                }
            }
        }
    }
    n
}
fn split(n: &Net, axis: usize) -> [Net; 2] {
    let (nu, nv) = (n.len(), n[0].len());
    let mut out = [vec![vec![Q::zero(); nv]; nu], vec![vec![Q::zero(); nv]; nu]];
    for line in 0..if axis == 0 { nv } else { nu } {
        let count = if axis == 0 { nu } else { nv };
        let mut p = (0..count)
            .map(|k| {
                if axis == 0 {
                    n[k][line].clone()
                } else {
                    n[line][k].clone()
                }
            })
            .collect::<Vec<_>>();
        for r in 0..count {
            if axis == 0 {
                out[0][r][line] = p[0].clone();
                out[1][count - r - 1][line] = p[count - r - 1].clone();
            } else {
                out[0][line][r] = p[0].clone();
                out[1][line][count - r - 1] = p[count - r - 1].clone();
            }
            for k in 0..count - r - 1 {
                p[k] = (&p[k] + &p[k + 1]) / Q::from_integer(2.into());
            }
        }
    }
    out
}
/// Exact normalized polynomial square, with a strictly positive scale.
/// No square root rounding: normalize the leading coefficient to one, reconstruct
/// every coefficient, then compare the complete coefficient identity.
fn square(p: &P) -> Result<bool> {
    let Some((&leading, scale)) = p.last_key_value() else {
        return Ok(false);
    };
    if !scale.is_positive() || leading.iter().any(|v| v % 2 != 0) {
        return Ok(false);
    }
    let lead = [leading[0] / 2, leading[1] / 2];
    let mut h = P::from([(lead, Q::one())]);
    for _ in 0..153 {
        let mut r = p.iter().map(|(e, v)| (*e, v / scale)).collect::<P>();
        for (e, v) in mul(&h, &h)? {
            add(&mut r, e, -v);
        }
        let Some((&e, v)) = r.last_key_value() else {
            return Ok(true);
        };
        if e[0] < lead[0] || e[1] < lead[1] {
            return Ok(false);
        }
        let t = [e[0] - lead[0], e[1] - lead[1]];
        if t >= lead || h.contains_key(&t) {
            return Ok(false);
        }
        h.insert(t, v / Q::from_integer(2.into()));
    }
    Ok(false)
}
fn prove(p: &P, limits: GraphOrderLimits) -> Result<(usize, bool)> {
    check(
        (1..=65536).contains(&limits.max_cells) && limits.max_depth <= 32,
        "Invalid polynomial separator work",
    )?;
    if square(p)? {
        return Ok((1, true));
    }
    let mut pending = vec![(to_net(p), 0)];
    let mut cells = 0;
    while let Some((n, depth)) = pending.pop() {
        cells += 1;
        check(
            cells <= limits.max_cells,
            "Polynomial separator cell budget exhausted",
        )?;
        if n.iter().flatten().all(|v| !v.is_negative()) {
            continue;
        }
        check(
            !n.iter().flatten().all(|v| v.is_negative()),
            "Polynomial separator has negative material gap",
        )?;
        check(
            depth < limits.max_depth,
            "Polynomial separator sign remains unproved",
        )?;
        let axis = if n.len() == 1 {
            1
        } else if n[0].len() == 1 {
            0
        } else {
            depth % 2
        };
        for child in split(&n, axis).into_iter().rev() {
            pending.push((child, depth + 1));
        }
    }
    Ok((cells, false))
}
/// Check the complete graph gap on the FIRST carrier's normalized rectangle.
/// A nonzero square identity additionally proves its algebraic contact zero set
/// globally, including cusps, crossing contacts and nonisolated tangencies.
pub fn certify_polynomial_graph_order(
    source: &FiberSources,
    limits: GraphOrderLimits,
) -> Result<QualifiedGraphMaterialOrder> {
    // A sign along a vertical face is not an order between two graph barriers.
    inverse_xy(source.first())?;
    let p = gap(source.first(), source.second())?;
    check(
        !p.is_empty(),
        "Coincident whole carriers need contact-face cancellation",
    )?;
    let (cells, square) = prove(&p, limits)?;
    Ok(QualifiedGraphMaterialOrder {
        source: source.clone(),
        limits,
        cells,
        square,
    })
}
impl QualifiedGraphMaterialOrder {
    pub fn source(&self) -> &FiberSources {
        &self.source
    }
    pub fn cells(&self) -> usize {
        self.cells
    }
    pub fn has_global_square_identity(&self) -> bool {
        self.square
    }
    pub fn reverify(&self) -> Result<()> {
        let c = certify_polynomial_graph_order(&self.source, self.limits)?;
        check(
            c.cells == self.cells && c.square == self.square,
            "Polynomial graph material order changed",
        )
    }
}
/// Exact affine XY hull inclusion, needed when a rectangle-only order proof is
/// used for a bounded material. All original face parameters are included.
pub fn prove_graph_projection_inside(barrier: &Surface, face: &Surface) -> Result<()> {
    let map = map_to(barrier, face)?;
    for a in map {
        for u in [Q::zero(), Q::one()] {
            for v in [Q::zero(), Q::one()] {
                let value = &a[0] + &a[1] * &u + &a[2] * v;
                check(
                    Q::zero() <= value && value <= Q::one(),
                    "Original boundary leaves graph separator rectangle",
                )?;
            }
        }
    }
    Ok(())
}
/// Every point of an ORIGINAL full face must lie below/above the chosen graph.
/// Positive-weight polynomial identities and exact sign certificates authorize
/// the claim; face samples or result triangulations do not participate.
pub fn prove_polynomial_graph_boundary_side(
    barrier: &Surface,
    face: &Surface,
    below: bool,
    limits: GraphOrderLimits,
) -> Result<usize> {
    let mut p = composed_height(barrier, face)?;
    for (e, v) in bernstein_power(face, 2)? {
        add(&mut p, e, -v);
    }
    if !below {
        for v in p.values_mut() {
            *v = -v.clone();
        }
    }
    prove(&p, limits).map(|r| r.0)
}

/// Materialize a conditioning system ONLY when every transformed coefficient
/// is exactly binary64. T(x,y,z)=(x,y,z-H_first(x,y)) is a global polynomial
/// diffeomorphism (det DT=1). It preserves every parameter root, multiplicity
/// and original-world branch; it changes no authored carrier in the Model.
pub(super) fn exact_graph_shear(first: &Surface, second: &Surface) -> Result<[Surface; 2]> {
    use num_traits::ToPrimitive;
    let own = composed_height(first, first)?;
    check(
        own == bernstein_power(first, 2)?,
        "Graph inverse/composition identity failed",
    )?;
    let mut difference = bernstein_power(second, 2)?;
    for (e, v) in composed_height(first, second)? {
        add(&mut difference, e, -v);
    }
    check(
        difference
            .keys()
            .all(|e| e[0] <= second.degree_u && e[1] <= second.degree_v),
        "Exact sheared height exceeds original tensor degrees",
    )?;
    let mut a = first.clone();
    let mut b = second.clone();
    for p in a.control_points.iter_mut().flatten() {
        p[2] = 0.;
    }
    for i in 0..=second.degree_u {
        for j in 0..=second.degree_v {
            let mut z = Q::zero();
            for (e, v) in &difference {
                if e[0] <= i && e[1] <= j {
                    z += v
                        * Q::new(c(i, e[0]).into(), c(second.degree_u, e[0]).into())
                        * Q::new(c(j, e[1]).into(), c(second.degree_v, e[1]).into());
                }
            }
            let value = z
                .to_f64()
                .ok_or_else(|| crate::input("Sheared height exceeds binary64 range"))?;
            check(
                q(value)? == z,
                "Sheared height needs rounding; refuse coordinate conditioning",
            )?;
            b.control_points[i][j][2] = value;
        }
    }
    a.validate()?;
    b.validate()?;
    Ok([a, b])
}
