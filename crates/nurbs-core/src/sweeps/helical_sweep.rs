//! Quintic helical sweep about the global Z axis, with an ideal continuous remainder.
use crate::{Result, check, curve::Curve, surface::Surface};
#[derive(Clone, Debug)]
pub struct Approximation {
    pub surface: Surface,
    pub spans: usize,
    pub budget: f64,
    /// Covers real-arithmetic Hermite truncation, excluding binary64 rounding.
    pub real_arithmetic_error_estimate: f64,
    pub rounding_certified: bool,
}
/// S(u,v)=Rz(start_angle+twist*v) C(u)+[0,0,height*v], v in [0,1].
/// Original U basis and positive weights are retained. Maximum six quintic
/// V spans (31 columns); refusal never relaxes the requested error budget.
pub fn approximate(
    c: &Curve,
    height: f64,
    start_angle: f64,
    twist: f64,
    budget: f64,
) -> Result<Approximation> {
    c.validate()?;
    check(
        c.control_points[0].len() == 3 && c.control_points.len() <= 32,
        "Helical sweep requires a 3D curve with at most 32 controls",
    )?;
    check(
        [height, start_angle, twist, budget]
            .iter()
            .all(|x| x.is_finite())
            && budget > 0.,
        "Helical sweep requires finite parameters and a positive error budget",
    )?;
    let radius = c
        .control_points
        .iter()
        .map(|p| p[0].hypot(p[1]))
        .fold(0., f64::max);
    let jet = |p: &[f64], v: f64| {
        if radius == 0. {
            return ([0., 0., p[2] + height * v], [0., 0., height], [0.; 3]);
        }
        let (sin, cos) = (start_angle + twist * v).sin_cos();
        let x = cos * p[0] - sin * p[1];
        let y = sin * p[0] + cos * p[1];
        (
            [x, y, p[2] + height * v],
            [-twist * y, twist * x, height],
            [-twist * twist * x, -twist * twist * y, 0.],
        )
    };
    if twist == 0. || radius == 0. {
        let mut a = c.clone();
        let mut b = c.clone();
        for (i, p) in c.control_points.iter().enumerate() {
            a.control_points[i] = jet(p, 0.).0.to_vec();
            b.control_points[i] = jet(p, 1.).0.to_vec();
        }
        return Ok(Approximation {
            surface: crate::surface::loft(&[a, b])?,
            spans: 1,
            budget,
            real_arithmetic_error_estimate: 0.,
            rounding_certified: false,
        });
    }
    // Each rotating control has ||P^(6)|| = radius_i*|twist|^6.
    // Positive rational U basis coefficients form a convex combination,
    // so the worst control error bounds the complete original U domain.
    let (spans,estimate) = (1..=6).find_map(|n| {
        let turn = twist/n as f64;
        let t2 = turn*turn;
        let e = 2_f64.sqrt()*radius*t2*t2*t2/46080.;
        (e.is_finite() && e>0. && e<=budget).then_some((n,e))
    }).ok_or_else(|| crate::resource("Helical sweep requires more than six quintic spans or an unrepresentable error estimate"))?;
    let h = 1. / spans as f64;
    let mut controls = Vec::with_capacity(c.control_points.len());
    for p in &c.control_points {
        let mut row = Vec::with_capacity(5 * spans + 1);
        for i in 0..spans {
            let (a, da, dda) = jet(p, i as f64 / spans as f64);
            let (b, db, ddb) = jet(p, (i + 1) as f64 / spans as f64);
            if i == 0 {
                row.push(a.to_vec());
            }
            row.push((0..3).map(|j| a[j] + h * da[j] / 5.).collect());
            row.push(
                (0..3)
                    .map(|j| a[j] + 2. * h * da[j] / 5. + h * h * dda[j] / 20.)
                    .collect(),
            );
            row.push(
                (0..3)
                    .map(|j| b[j] - 2. * h * db[j] / 5. + h * h * ddb[j] / 20.)
                    .collect(),
            );
            row.push((0..3).map(|j| b[j] - h * db[j] / 5.).collect());
            row.push(b.to_vec());
        }
        controls.push(row);
    }
    let mut knots = vec![0.; 6];
    for i in 1..=spans {
        knots.extend(std::iter::repeat_n(i as f64 / spans as f64, 5));
    }
    knots.push(1.);
    let surface = Surface {
        degree_u: c.degree,
        degree_v: 5,
        knots_u: c.knots.clone(),
        knots_v: knots,
        weights: c.weights.iter().map(|&w| vec![w; 5 * spans + 1]).collect(),
        control_points: controls,
        periodic_u: c.periodic,
        periodic_v: false,
    };
    surface.validate()?;
    Ok(Approximation {
        surface,
        spans,
        budget,
        real_arithmetic_error_estimate: estimate,
        rounding_certified: false,
    })
}
