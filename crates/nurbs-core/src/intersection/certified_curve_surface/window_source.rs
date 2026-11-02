//! Original surface iso bounds without materializing rounded curve controls.
use super::*;
use num_rational::BigRational as Q;
use num_traits::{FromPrimitive, ToPrimitive};
pub(super) enum Source<'a> {
    Curve(&'a Curve),
    Iso {
        surface: &'a Surface,
        axis: usize,
        fixed_span: usize,
        fixed: [f64; 2],
    },
}
impl Source<'_> {
    pub fn valid(&self) -> bool {
        match self {
            Self::Curve(c) => c.control_points[0].len() == 3 && !c.periodic,
            Self::Iso { surface: s, .. } => {
                s.control_points[0][0].len() == 3 && !s.periodic_u && !s.periodic_v
            }
        }
    }
    pub fn degree(&self) -> usize {
        match self {
            Self::Curve(c) => c.degree,
            Self::Iso {
                surface: s, axis, ..
            } => {
                if *axis == 0 {
                    s.degree_u
                } else {
                    s.degree_v
                }
            }
        }
    }
    pub fn count(&self) -> usize {
        match self {
            Self::Curve(c) => c.control_points.len(),
            Self::Iso {
                surface: s, axis, ..
            } => {
                if *axis == 0 {
                    s.control_points.len()
                } else {
                    s.control_points[0].len()
                }
            }
        }
    }
    pub fn knots(&self) -> &[f64] {
        match self {
            Self::Curve(c) => &c.knots,
            Self::Iso {
                surface: s, axis, ..
            } => {
                if *axis == 0 {
                    &s.knots_u
                } else {
                    &s.knots_v
                }
            }
        }
    }
    pub fn domain(&self) -> [f64; 2] {
        [self.knots()[self.degree()], self.knots()[self.count()]]
    }
    fn homogeneous(&self, span: usize, d: [f64; 2]) -> Result<Vec<[I; 4]>> {
        let Self::Iso {
            surface,
            axis,
            fixed_span,
            fixed,
        } = self
        else {
            unreachable!()
        };
        let (spans, domain) = if *axis == 0 {
            ([span, *fixed_span], [d, *fixed])
        } else {
            ([*fixed_span, span], [*fixed, d])
        };
        let net = crate::curve_surface_composition::surface_net_on(surface, spans, domain)?;
        // The exact fixed parameter lies in `fixed`. Hull transverse Bernstein
        // coefficients: every resulting interval encloses the coefficient of
        // the ONE exact iso, including when that parameter is not binary64.
        Ok((0..=self.degree())
            .map(|i| {
                std::array::from_fn(|k| {
                    if *axis == 0 {
                        hull(net[i].iter().map(|p| p[k]))
                    } else {
                        hull(net.iter().map(|row| row[i][k]))
                    }
                })
            })
            .collect())
    }
    pub fn field(&self, s: &Surface, ids: [usize; 3], domain: Box3) -> Result<Net> {
        if let Self::Curve(c) = self {
            return field_net(c, s, ids, domain);
        }
        let ch = self.homogeneous(ids[0], domain[0])?;
        let sh = crate::curve_surface_composition::surface_net_on(
            s,
            [ids[1], ids[2]],
            [domain[1], domain[2]],
        )?;
        let dims = [ch.len(), sh.len(), sh[0].len()];
        let mut data = Vec::with_capacity(dims.iter().product());
        for cp in ch {
            for row in &sh {
                for sp in row {
                    let mut f = [I::point(0.); 3];
                    for k in 0..3 {
                        f[k] = cp[k].mul(sp[3])?.sub(sp[k].mul(cp[3])?)?;
                    }
                    data.push(f);
                }
            }
        }
        Ok(Net { dims, data })
    }
    pub fn position(&self, span: usize, d: [f64; 2]) -> Result<Vec<I>> {
        if let Self::Curve(c) = self {
            return crate::curve_distance::enclosure(c, span, I::new(d[0], d[1])?);
        }
        let h = self.homogeneous(span, d)?;
        let w = hull(h.iter().map(|p| p[3]));
        (0..3)
            .map(|k| hull(h.iter().map(|p| p[k])).div(w))
            .collect()
    }
}
/// First original iso/surface root in a natural parameter window. `parameter`
/// is normalized on the transverse source axis and interpreted exactly.
/// No rounded extracted curve, trim admission or surface completeness implied.
fn iso_search(
    s: &Surface,
    axis: usize,
    parameter: f64,
    window: Box3,
    max_cells: usize,
    stop_first: bool,
) -> Result<window::WindowReport> {
    s.validate()?;
    check(
        axis < 2 && parameter.is_finite() && parameter > 0. && parameter < 1.,
        "Invalid source iso",
    )?;
    let (k, p, n) = if axis == 0 {
        (&s.knots_v, s.degree_v, s.control_points[0].len())
    } else {
        (&s.knots_u, s.degree_u, s.control_points.len())
    };
    let q = |x| Q::from_f64(x).unwrap();
    let exact = q(k[p]) + (q(k[n]) - q(k[p])) * q(parameter);
    let span = (p..n)
        .find(|&i| q(k[i]) <= exact && exact < q(k[i + 1]))
        .ok_or_else(|| crate::input("No source iso span"))?;
    let f = exact
        .to_f64()
        .ok_or_else(|| crate::numeric_err("No finite source iso"))?;
    let fixed = [
        if q(f) > exact { f.next_down() } else { f },
        if q(f) < exact { f.next_up() } else { f },
    ];
    check(
        fixed[0] >= k[span] && fixed[1] <= k[span + 1],
        "Iso enclosure leaves source span",
    )?;
    window::search(
        Source::Iso {
            surface: s,
            axis,
            fixed_span: span,
            fixed,
        },
        s,
        window,
        max_cells,
        stop_first,
    )
}

/// First certified original iso/surface root; unvisited windows remain unresolved.
pub fn first_iso_in_window(
    s: &Surface,
    axis: usize,
    parameter: f64,
    window: Box3,
    max_cells: usize,
) -> Result<window::WindowReport> {
    iso_search(s, axis, parameter, window, max_cells, true)
}
/// Complete requested-window search on original iso/surface coefficients.
/// This proves neither whole-surface coverage nor trim/Body admission.
pub fn all_iso_in_window(
    s: &Surface,
    axis: usize,
    parameter: f64,
    window: Box3,
    max_cells: usize,
) -> Result<window::WindowReport> {
    iso_search(s, axis, parameter, window, max_cells, false)
}
