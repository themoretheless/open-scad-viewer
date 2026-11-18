//! Exact native bilinear-plane charts and continuous edge/pcurve agreement.
//! A nonzero bounded seam is explicitly NOT an exact boundary identity.
use super::*;
use crate::profile_certificate::arithmetic as a;
use nurbs_core::surface::Surface;
pub mod pullback;
#[derive(Clone, Debug)]
pub struct Chart {
    surface: Surface,
    domain: [[Q; 2]; 2],
    coefficients: [[Q; 2]; 4],
    jacobian_sign: i8,
}
fn q(x: f64) -> Result<Q> {
    Q::from_float(x).ok_or_else(|| refused("Nonfinite native plane coefficient"))
}
pub fn chart(surface: &Surface) -> Result<Chart> {
    surface.validate()?;
    if surface.degree_u != 1
        || surface.degree_v != 1
        || surface.control_points.len() != 2
        || surface.control_points[0].len() != 2
        || surface.periodic_u
        || surface.periodic_v
        || surface
            .weights
            .iter()
            .flatten()
            .any(|w| *w <= 0. || *w != surface.weights[0][0])
    {
        return Err(refused(
            "Native cap needs a positive constant-weight bilinear chart",
        ));
    }
    let p = [
        &surface.control_points[0][0],
        &surface.control_points[1][0],
        &surface.control_points[0][1],
        &surface.control_points[1][1],
    ];
    if p.iter()
        .any(|p| p.len() != 3 || p[2] != surface.control_points[0][0][2])
    {
        return Err(refused("Native cap carrier is not a horizontal plane"));
    }
    let points = p.map(|p| [q(p[0]).unwrap(), q(p[1]).unwrap()]);
    let coefficients = std::array::from_fn(|i| {
        std::array::from_fn(|axis| match i {
            0 => points[0][axis].clone(),
            1 => &points[1][axis] - &points[0][axis],
            2 => &points[2][axis] - &points[0][axis],
            _ => &points[3][axis] - &points[1][axis] - &points[2][axis] + &points[0][axis],
        })
    });
    let cross = |x: &[Q; 2], y: &[Q; 2]| &x[0] * &y[1] - &x[1] * &y[0];
    let base = cross(&coefficients[1], &coefficients[2]);
    let du = cross(&coefficients[1], &coefficients[3]);
    let dv = cross(&coefficients[3], &coefficients[2]);
    let corners = [base.clone(), &base + &du, &base + &dv, &base + &du + &dv];
    let jacobian_sign = if corners.iter().all(|v| v > &Q::zero()) {
        1
    } else if corners.iter().all(|v| v < &Q::zero()) {
        -1
    } else {
        return Err(refused(
            "Native bilinear cap chart is not globally injective/regular",
        ));
    };
    // The determinant is affine on the unit square. These four strict signs
    // also give the four strict convex boundary turns, hence a global chart.
    let domain = [
        [
            q(surface.knots_u[surface.degree_u])?,
            q(surface.knots_u[surface.control_points.len()])?,
        ],
        [
            q(surface.knots_v[surface.degree_v])?,
            q(surface.knots_v[surface.control_points[0].len()])?,
        ],
    ];
    if domain.iter().any(|d| d[0] >= d[1]) {
        return Err(refused("Empty native plane chart domain"));
    }
    Ok(Chart {
        surface: surface.clone(),
        domain,
        coefficients,
        jacobian_sign,
    })
}
fn choose(n: usize, k: usize) -> Q {
    let mut v = 1usize;
    for i in 0..k {
        v = v * (n - i) / (i + 1);
    }
    Q::from_integer(v.into())
}
fn product(x: &[Q], y: &[Q]) -> Vec<Q> {
    let (n, m) = (x.len() - 1, y.len() - 1);
    let mut out = vec![Q::zero(); n + m + 1];
    for i in 0..=n {
        for j in 0..=m {
            out[i + j] += &x[i] * &y[j] * choose(n, i) * choose(m, j) / choose(n + m, i + j);
        }
    }
    out
}
fn nonnegative(p: &[Q]) -> Result<bool> {
    let Some(first) = p.iter().find(|v| !v.is_zero()) else {
        return Ok(true);
    };
    if first < &Q::zero() || a::at(p, &Q::one()) < Q::zero() {
        return Ok(false);
    }
    if p.len() < 2 {
        return Ok(true);
    }
    let mut g = a::gcd(p, &a::derivative(p))?;
    let mut w = a::quotient(p, &g)?;
    let mut multiplicity = 1;
    while w.len() > 1 {
        let y = a::gcd(&w, &g)?;
        let factor = a::quotient(&w, &y)?;
        if multiplicity % 2 == 1 && a::algebraic_events::count(&factor, &Q::zero(), &Q::one())? > 0
        {
            return Ok(false);
        }
        w = y;
        g = a::quotient(&g, &w)?;
        multiplicity += 1;
        if multiplicity > p.len() {
            return Err(refused("Native cap sign multiplicity census exhausted"));
        }
    }
    Ok(true)
}
fn inside(net: &[[Q; 3]], domain: &[[Q; 2]; 2]) -> Result<()> {
    for axis in 0..2 {
        for side in 0..2 {
            let coefficients = net
                .iter()
                .map(|h| {
                    let v = &h[axis] - &domain[axis][side] * &h[2];
                    [if side == 0 { v } else { -v }, Q::zero(), Q::zero()]
                })
                .collect::<Vec<_>>();
            if !nonnegative(&a::power(&coefficients, 0))? {
                return Err(refused("Actual native pcurve leaves the cap chart"));
            }
        }
    }
    Ok(())
}
impl Chart {
    pub fn reverify(&self) -> Result<()> {
        let current = chart(&self.surface)?;
        if current.domain != self.domain || current.coefficients != self.coefficients
            || current.jacobian_sign != self.jacobian_sign {
            return Err(refused("Native cap chart replay changed"));
        }
        Ok(())
    }
    pub fn surface(&self) -> &Surface {
        &self.surface
    }
    pub fn jacobian_sign(&self) -> i8 {
        self.jacobian_sign
    }
    pub fn mixed_xy_coefficient(&self) -> [ExactRatio; 2] {
        self.coefficients[3].each_ref().map(ratio)
    }
    fn map(&self, net: &[[Q; 3]]) -> Result<Vec<[Q; 3]>> {
        inside(net, &self.domain)?;
        let w = net.iter().map(|h| h[2].clone()).collect::<Vec<_>>();
        let u = net
            .iter()
            .map(|h| {
                (&h[0] - &self.domain[0][0] * &h[2]) / (&self.domain[0][1] - &self.domain[0][0])
            })
            .collect::<Vec<_>>();
        let v = net
            .iter()
            .map(|h| {
                (&h[1] - &self.domain[1][0] * &h[2]) / (&self.domain[1][1] - &self.domain[1][0])
            })
            .collect::<Vec<_>>();
        let terms = [
            product(&w, &w),
            product(&u, &w),
            product(&v, &w),
            product(&u, &v),
        ];
        Ok((0..terms[0].len())
            .map(|i| {
                [
                    (0..4)
                        .map(|k| &self.coefficients[k][0] * &terms[k][i])
                        .sum(),
                    (0..4)
                        .map(|k| &self.coefficients[k][1] * &terms[k][i])
                        .sum(),
                    terms[0][i].clone(),
                ]
            })
            .collect())
    }
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TraceCell {
    normalized: [Q; 2],
    net: Vec<[Q; 3]>,
    error_squared: Q,
}
impl TraceCell {
    pub fn normalized_interval(&self) -> [ExactRatio; 2] {
        self.normalized.each_ref().map(ratio)
    }
    pub fn homogeneous_controls(&self) -> Vec<[ExactRatio; 3]> {
        self.net.iter().map(|h| h.each_ref().map(ratio)).collect()
    }
    pub fn error_squared_bound(&self) -> ExactRatio {
        ratio(&self.error_squared)
    }
}
#[derive(Clone, Debug)]
pub struct Boundary {
    chart: Chart,
    uv: Restriction,
    world: Restriction,
    tolerance: Q,
    cells: Vec<TraceCell>,
    maximum: Q,
}
fn derive(
    chart: &Chart,
    uv: &Restriction,
    world: &Restriction,
    tolerance: &Q,
) -> Result<(Vec<TraceCell>, Q)> {
    uv.reverify()?;
    world.reverify()?;
    let mut cuts = vec![Q::zero(), Q::one()];
    for r in [uv, world] {
        for s in &r.spans {
            for t in &s.range {
                cuts.push((t - &r.range[0]) / (&r.range[1] - &r.range[0]));
            }
        }
    }
    cuts.sort();
    cuts.dedup();
    if cuts.len() > 129 || tolerance < &Q::zero() {
        return Err(refused(
            "Native cap common-span budget or tolerance invalid",
        ));
    }
    let mut cells = Vec::new();
    let mut maximum = Q::zero();
    for cut in cuts.windows(2) {
        let extract = |r: &Restriction| {
            let t = cut
                .iter()
                .map(|s| &r.range[0] + s * (&r.range[1] - &r.range[0]))
                .collect::<Vec<_>>();
            extraction::interval(&r.source, &t[0], &t[1])
        };
        let native = extract(world)?;
        let mapped = chart.map(&extract(uv)?)?;
        let wa = native.iter().map(|h| h[2].clone()).collect::<Vec<_>>();
        let wb = mapped.iter().map(|h| h[2].clone()).collect::<Vec<_>>();
        let denominator = product(&wa, &wb);
        let errors: [Vec<Q>; 2] = std::array::from_fn(|axis| {
            let a = product(
                &native.iter().map(|h| h[axis].clone()).collect::<Vec<_>>(),
                &wb,
            );
            let b = product(
                &mapped.iter().map(|h| h[axis].clone()).collect::<Vec<_>>(),
                &wa,
            );
            a.iter().zip(b).map(|(a, b)| a - b).collect()
        });
        let mut bound = Q::zero();
        for i in 0..denominator.len() {
            if denominator[i] <= Q::zero() {
                return Err(refused(
                    "Native cap seam has nonpositive rational denominator",
                ));
            }
            let x = &errors[0][i] / &denominator[i];
            let y = &errors[1][i] / &denominator[i];
            bound = bound.max(&x * &x + &y * &y);
        }
        // Positive Bernstein denominator: the complete displacement is a
        // convex combination of these rational vector control differences.
        maximum = maximum.max(bound.clone());
        cells.push(TraceCell {
            normalized: [cut[0].clone(), cut[1].clone()],
            net: mapped,
            error_squared: bound,
        });
    }
    if maximum > tolerance * tolerance {
        return Err(refused(
            "Native cap continuous seam bound exceeds model tolerance",
        ));
    }
    Ok((cells, maximum))
}
pub fn qualify(surface: &Surface, uv: &Curve, world: &Curve, tolerance: f64) -> Result<Boundary> {
    let chart = chart(surface)?;
    let uv = restrict_binary64(uv, uv.domain())?;
    let world = restrict_binary64(world, world.domain())?;
    let tolerance = q(tolerance)?;
    let (cells, maximum) = derive(&chart, &uv, &world, &tolerance)?;
    Ok(Boundary {
        chart,
        uv,
        world,
        tolerance,
        cells,
        maximum,
    })
}
impl Boundary {
    pub fn chart(&self) -> &Chart {
        &self.chart
    }
    pub fn pcurve(&self) -> &Curve {
        self.uv.source()
    }
    pub fn world_curve(&self) -> &Curve {
        self.world.source()
    }
    pub fn cells(&self) -> &[TraceCell] {
        &self.cells
    }
    pub fn error_squared_bound(&self) -> ExactRatio {
        ratio(&self.maximum)
    }
    pub fn exact(&self) -> bool {
        self.maximum.is_zero()
    }
    pub fn reverify(&self) -> Result<()> {
        self.chart.reverify()?;
        let chart = chart(&self.chart.surface)?;
        let (cells, maximum) = derive(&chart, &self.uv, &self.world, &self.tolerance)?;
        if cells != self.cells || maximum != self.maximum {
            return Err(refused("Native cap continuous boundary replay changed"));
        }
        Ok(())
    }
}
