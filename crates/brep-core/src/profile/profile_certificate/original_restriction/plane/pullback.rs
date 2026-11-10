//! Unique inverse-chart boundary expressions on the UNCHANGED native carrier.
//! The world curve is primary. No rounded UV controls are manufactured.
use super::*;
use num_traits::Signed;
#[derive(Clone, Debug)]
pub struct Pullback {
    chart: Chart,
    world: Restriction,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct UvSpan {
    range: [Q; 2],
    net: Vec<[Q; 3]>,
}
impl UvSpan {
    pub fn parameter_range(&self) -> [ExactRatio; 2] {
        self.range.each_ref().map(ratio)
    }
    pub fn homogeneous_controls(&self) -> Vec<[ExactRatio; 3]> {
        self.net.iter().map(|h| h.each_ref().map(ratio)).collect()
    }
}
fn cross(x: &[Q; 2], y: &[Q; 2]) -> Q {
    &x[0] * &y[1] - &x[1] * &y[0]
}
fn contained(chart: &Chart, world: &Restriction) -> Result<()> {
    chart.reverify()?;
    world.reverify()?;
    let c = &chart.coefficients;
    let vertices = [
        c[0].clone(),
        std::array::from_fn(|k| &c[0][k] + &c[1][k]),
        std::array::from_fn(|k| &c[0][k] + &c[1][k] + &c[2][k] + &c[3][k]),
        std::array::from_fn(|k| &c[0][k] + &c[2][k]),
    ];
    for span in &world.spans {
        for i in 0..4 {
            let p = &vertices[i];
            let next = &vertices[(i + 1) % 4];
            let d = [&next[0] - &p[0], &next[1] - &p[1]];
            let net = span
                .net
                .iter()
                .map(|h| {
                    let value = (&d[0] * (&h[1] - &p[1] * &h[2]) - &d[1] * (&h[0] - &p[0] * &h[2]))
                        * Q::from_integer(chart.jacobian_sign.into());
                    [value, Q::zero(), Q::zero()]
                })
                .collect::<Vec<_>>();
            if !nonnegative(&a::power(&net, 0))? {
                return Err(refused(
                    "Original world rim leaves the actual native cap image",
                ));
            }
        }
    }
    Ok(())
}
pub fn qualify(surface: &Surface, world: &Curve) -> Result<Pullback> {
    let chart = chart(surface)?;
    let world = restrict_binary64(world, world.domain())?;
    contained(&chart, &world)?;
    Ok(Pullback { chart, world })
}
fn native_box(chart: &Chart, p: [[Q; 2]; 2]) -> [[Q; 2]; 2] {
    std::array::from_fn(|k| {
        p[k].each_ref()
            .map(|q| &chart.domain[k][0] + q * (&chart.domain[k][1] - &chart.domain[k][0]))
    })
}
fn divide(n: [Q; 2], d: [Q; 2]) -> [Q; 2] {
    let values = [&n[0] / &d[0], &n[0] / &d[1], &n[1] / &d[0], &n[1] / &d[1]];
    [
        values.iter().min().unwrap().clone(),
        values.iter().max().unwrap().clone(),
    ]
}
impl Pullback {
    pub fn chart(&self) -> &Chart {
        &self.chart
    }
    pub fn world_curve(&self) -> &Curve {
        self.world.source()
    }
    pub fn reverify(&self) -> Result<()> {
        contained(&self.chart, &self.world)
    }
    /// Exact finite rational UV controls for an affine carrier; otherwise the
    /// same owner defines a unique implicit bilinear inverse, evaluated below.
    pub fn affine_spans(&self) -> Result<Option<Vec<UvSpan>>> {
        self.reverify()?;
        let c = &self.chart.coefficients;
        if c[3].iter().any(|v| !v.is_zero()) {
            return Ok(None);
        }
        let det = cross(&c[1], &c[2]);
        Ok(Some(
            self.world
                .spans
                .iter()
                .map(|s| UvSpan {
                    range: s.range.clone(),
                    net: s
                        .net
                        .iter()
                        .map(|h| {
                            let p = [&h[0] - &c[0][0] * &h[2], &h[1] - &c[0][1] * &h[2]];
                            let u = cross(&p, &c[2]) / &det;
                            let v = cross(&c[1], &p) / &det;
                            [
                                &self.chart.domain[0][0] * &h[2]
                                    + u * (&self.chart.domain[0][1] - &self.chart.domain[0][0]),
                                &self.chart.domain[1][0] * &h[2]
                                    + v * (&self.chart.domain[1][1] - &self.chart.domain[1][0]),
                                h[2].clone(),
                            ]
                        })
                        .collect(),
                })
                .collect(),
        ))
    }
    /// Certified enclosure in original UV units. Exhaustion returns no box.
    pub fn inverse_box(
        &self,
        parameter: ExactRatio,
        width: ExactRatio,
        refinements: usize,
    ) -> Result<InverseBox> {
        self.reverify()?;
        let width = decode_ratio(&width)?;
        if width <= Q::zero() || refinements > 512 {
            return Err(refused("Invalid native inverse width or allowance"));
        }
        let value = self.world.evaluate_natural(parameter.clone())?;
        let point = [decode_ratio(&value[0])?, decode_ratio(&value[1])?];
        let c = &self.chart.coefficients;
        let p = [&point[0] - &c[0][0], &point[1] - &c[0][1]];
        let bounds = if c[3].iter().all(Zero::is_zero) {
            let det = cross(&c[1], &c[2]);
            let uv = [cross(&p, &c[2]) / &det, cross(&c[1], &p) / &det];
            native_box(&self.chart, uv.map(|v| [v.clone(), v]))
        } else {
            let f = vec![
                cross(&p, &c[2]),
                cross(&p, &c[3]) - cross(&c[1], &c[2]),
                -cross(&c[1], &c[3]),
            ];
            // Trim the possible zero quadratic coefficient without floating tests.
            let f = a::sub(&f, &[]);
            let mut candidates = Vec::new();
            for mut root in a::algebraic_events::isolates(
                &f,
                Q::zero(),
                Q::one(),
                &Q::new(1.into(), 1024.into()),
            )? {
                let mut resolved = None;
                for step in 0..=refinements {
                    let mut best: Option<(Q, [Q; 2])> = None;
                    for axis in 0..2 {
                        let denominator = vec![c[2][axis].clone(), c[3][axis].clone()];
                        let d = a::parameter_order::range(&denominator, &root);
                        if d[0] <= Q::zero() && d[1] >= Q::zero() {
                            continue;
                        }
                        let score = d[0].clone().abs().min(d[1].clone().abs());
                        let numerator = vec![p[axis].clone(), -c[1][axis].clone()];
                        let v = if a::event_coordinates::zero_at(&numerator, &f, &root)? {
                            [Q::zero(), Q::zero()]
                        } else if a::event_coordinates::zero_at(
                            &a::sub(&numerator, &denominator),
                            &f,
                            &root,
                        )? {
                            [Q::one(), Q::one()]
                        } else {
                            divide(a::parameter_order::range(&numerator, &root), d)
                        };
                        if best.as_ref().is_none_or(|(s, _)| score > *s) {
                            best = Some((score, v));
                        }
                    }
                    if let Some((_, v)) = best {
                        if v[1] < Q::zero() || v[0] > Q::one() {
                            resolved = Some(None);
                            break;
                        }
                        if v[0] >= Q::zero() && v[1] <= Q::one() {
                            let bounds = native_box(&self.chart, [root.clone(), v]);
                            if bounds.iter().all(|b| &b[1] - &b[0] <= width) {
                                resolved = Some(Some(bounds));
                                break;
                            }
                        }
                    }
                    if step < refinements {
                        a::parameter_order::refine(&f, &mut root)?;
                    }
                }
                match resolved {
                    Some(Some(b)) => candidates.push(b),
                    Some(None) => {}
                    None => {
                        return Err(refused(
                            "Native bilinear inverse remains unresolved within allowance",
                        ));
                    }
                }
            }
            if candidates.len() != 1 {
                return Err(refused(
                    "Native inverse does not own one unique domain root",
                ));
            }
            candidates.remove(0)
        };
        Ok(InverseBox {
            owner: self.clone(),
            parameter,
            width,
            refinements,
            bounds,
        })
    }
}
#[derive(Clone, Debug)]
pub struct InverseBox {
    owner: Pullback,
    parameter: ExactRatio,
    width: Q,
    refinements: usize,
    bounds: [[Q; 2]; 2],
}
impl InverseBox {
    pub fn bounds(&self) -> [[ExactRatio; 2]; 2] {
        self.bounds.each_ref().map(|b| b.each_ref().map(ratio))
    }
    pub fn reverify(&self) -> Result<()> {
        let current =
            self.owner
                .inverse_box(self.parameter.clone(), ratio(&self.width), self.refinements)?;
        if current.bounds != self.bounds {
            return Err(refused("Native inverse enclosure replay changed"));
        }
        Ok(())
    }
}
