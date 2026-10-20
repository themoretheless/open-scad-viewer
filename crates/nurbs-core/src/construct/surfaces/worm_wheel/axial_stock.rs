//! Unique original flank/stock intersections at fixed physical wheel Y.
use super::*;

#[derive(Clone, Debug)]
pub struct AxialStockIntersection {
    spec: Spec,
    flank: Flank,
    y: [f64; 2],
    radius: f64,
    cells: usize,
    root: [f64; 2],
    derivative: [f64; 2],
    radial_upper: f64,
    cross_norm_lower: f64,
}
impl AxialStockIntersection {
    pub fn specification(&self) -> Spec {
        self.spec
    }
    pub fn flank(&self) -> Flank {
        self.flank
    }
    pub fn domain(&self) -> AxialDomain {
        AxialDomain {
            t: [0., 1.],
            y: self.y,
        }
    }
    pub fn radius(&self) -> f64 {
        self.radius
    }
    /// Original-profile U enclosing the unique stock crossing for every Y.
    pub fn u_bracket(&self) -> [f64; 2] {
        self.root
    }
    pub fn cross_norm_lower(&self) -> f64 {
        self.cross_norm_lower
    }
    pub fn stock_interior(&self) -> StockInterior {
        if self.flank == Flank::Descending {
            StockInterior::LowerU
        } else {
            StockInterior::HigherU
        }
    }
    pub fn reverify(&self) -> Result<()> {
        let fresh =
            axial_stock_intersection(self.spec, self.flank, self.y, self.radius, self.cells)?;
        check(
            fresh.root == self.root
                && fresh.derivative == self.derivative
                && fresh.radial_upper == self.radial_upper
                && fresh.cross_norm_lower == self.cross_norm_lower,
            "Original physical-Y stock proof changed",
        )
    }
    pub(super) fn validate_query(&self, d: AxialDomain) -> Result<()> {
        check(
            d.t.iter().chain(&d.y).all(|x| x.is_finite())
                && 0. <= d.t[0]
                && d.t[0] <= d.t[1]
                && d.t[1] <= 1.
                && self.y[0] <= d.y[0]
                && d.y[0] <= d.y[1]
                && d.y[1] <= self.y[1],
            "Axial stock query lies outside its original local-t/physical-Y domain",
        )
    }
    pub fn restrict_y(&self, y: [f64; 2]) -> Result<Self> {
        self.validate_query(AxialDomain { t: [0., 1.], y })?;
        axial_stock_intersection(self.spec, self.flank, y, self.radius, self.cells)
    }
    pub(super) fn root_jets(&self) -> Result<(I, I)> {
        let source = super::expression::prepare(self.spec, self.flank)?;
        let (_, g) = super::chart::axial_radial_jets(
            &source,
            AxialDomain {
                t: self.root,
                y: self.y,
            },
        )?;
        let derivative = g[0].intersect(self.derivative[0], self.derivative[1])?;
        Ok((derivative, neg(g[1])?.div_signed(derivative)?))
    }
    pub fn position_bounds(&self, d: AxialDomain) -> Result<[[f64; 2]; 3]> {
        self.validate_query(d)?;
        let root = if d.y == self.y {
            self.clone()
        } else {
            self.restrict_y(d.y)?
        };
        let (_, dy) = root.root_jets()?;
        let (mut p, _, derivative) =
            super::chart::axial_stock_jets(self.spec, self.flank, d, root.root, dy)?;
        let center = mid(iv(d.y)?);
        if d.y[1] - d.y[0] > 64. * f64::EPSILON * center.abs().max(1.) {
            // A whole-domain first-jet mean-value enclosure preserves the
            // implicit U(Y) correlation lost by direct coordinate boxes.
            let point = root.restrict_y([center, center])?;
            let (_, point_dy) = point.root_jets()?;
            let base = super::chart::axial_stock_jets(
                self.spec,
                self.flank,
                AxialDomain {
                    t: d.t,
                    y: [center, center],
                },
                point.root,
                point_dy,
            )?
            .0;
            let delta = iv(d.y)?.sub(I::point(center))?;
            for i in 0..3 {
                let mean = base[i].add(derivative[i].mul(delta)?)?;
                p[i] = p[i].intersect(mean.lo, mean.hi)?;
            }
        }
        Ok(p.map(pair))
    }
    /// True crest-to-stock chart; strict original radial progress proves source
    /// injectivity because physical Y fixes Y and squared radius fixes t.
    pub fn enclose(&self, d: AxialDomain) -> Result<AxialChartBounds> {
        self.validate_query(d)?;
        let root = if d.y == self.y {
            self.clone()
        } else {
            self.restrict_y(d.y)?
        };
        let (derivative, root_dy) = root.root_jets()?;
        let (p, dt, dy) =
            super::chart::axial_stock_jets(self.spec, self.flank, d, root.root, root_dy)?;
        let cross = [
            dt[1].mul(dy[2])?.sub(dt[2].mul(dy[1])?)?,
            dt[2].mul(dy[0])?.sub(dt[0].mul(dy[2])?)?,
            dt[0].mul(dy[1])?.sub(dt[1].mul(dy[0])?)?,
        ];
        Ok(AxialChartBounds {
            point: p.map(pair),
            dt: dt.map(pair),
            dy: dy.map(pair),
            sixth: [
                super::taylor::axial_stock_sixth(
                    self.spec, self.flank, d, root.root, derivative, 0,
                )?
                .map(pair),
                super::taylor::axial_stock_sixth(
                    self.spec, self.flank, d, root.root, derivative, 1,
                )?
                .map(pair),
            ],
            regularity_proven: root.cross_norm_lower > 0.
                || cross.iter().any(|x| x.lo > 0. || x.hi < 0.),
        })
    }
}

/// Full original-U x physical-Y derivative census, strict stock endpoint signs
/// and common root isolation. Retains the inside-blank side from crest to root.
/// No sampled sign tests, missing cells or caller-supplied monotonicity flags.
pub fn axial_stock_intersection(
    spec: Spec,
    flank: Flank,
    y: [f64; 2],
    radius: f64,
    cells: usize,
) -> Result<AxialStockIntersection> {
    check(
        radius.is_finite() && radius > 0. && radius <= 1e8 && (1..=4096).contains(&cells),
        "Axial stock requires positive bounded radius and 1..4096 U cells",
    )?;
    let p = worm::derive_parameters(spec.worm)?;
    let ratio = iv(y)?.div(I::new(p.root_radius[0], p.crest_radius[1])?)?;
    let beta = super::expression::asin_interval(ratio)?;
    super::chart::validate(
        spec,
        flank,
        ChartDomain {
            u: [0., 1.],
            beta: pair(beta),
        },
    )?;
    let source = super::expression::prepare(spec, flank)?;
    let r2 = I::point(radius).mul(I::point(radius))?;
    let mut left = y[0];
    let count = if y[0] == y[1] { 1 } else { 16 };
    let mut ys = Vec::with_capacity(count);
    for i in 1..=count {
        let right = if i == count {
            y[1]
        } else {
            y[0] + (y[1] - y[0]) * (i as f64 / count as f64)
        };
        check(left <= right, "Axial proof cover lost Y ordering")?;
        if left < right || count == 1 {
            ys.push([left, right]);
        }
        left = right;
    }
    let increasing = flank == Flank::Descending;
    let mut derivative_lo = f64::INFINITY;
    let mut derivative_hi = f64::NEG_INFINITY;
    let mut radial_upper = 0f64;
    let mut left = 0.;
    for i in 1..=cells {
        let right = if i == cells {
            1.
        } else {
            i as f64 / cells as f64
        };
        check(left < right, "Axial proof cover lost original U ordering")?;
        for &yp in &ys {
            let (r, d) = super::chart::axial_radial_jets(
                &source,
                AxialDomain {
                    t: [left, right],
                    y: yp,
                },
            )?;
            check(
                if increasing {
                    d[0].lo > 0.
                } else {
                    d[0].hi < 0.
                },
                "Original physical-Y stock radial derivative is not strictly separated",
            )?;
            derivative_lo = derivative_lo.min(d[0].lo);
            derivative_hi = derivative_hi.max(d[0].hi);
            radial_upper = radial_upper.max(r.hi.max(0.).sqrt().next_up());
        }
        left = right;
    }
    let residual = |u| -> Result<I> {
        let mut result = None::<I>;
        for &yp in &ys {
            let v = super::chart::axial_radial_jets(&source, AxialDomain { t: [u, u], y: yp })?
                .0
                .sub(r2)?;
            result = Some(match result {
                None => v,
                Some(old) => I::new(old.lo.min(v.lo), old.hi.max(v.hi))?,
            });
        }
        Ok(result.unwrap())
    };
    let before = |v: I| if increasing { v.hi < 0. } else { v.lo > 0. };
    let after = |v: I| if increasing { v.lo > 0. } else { v.hi < 0. };
    check(
        before(residual(0.)?) && after(residual(1.)?),
        "Stock signs do not retain an inside-blank crest and unique crossing for every Y",
    )?;
    let mut root = [0., 1.];
    for side in 0..2 {
        let mut search = root;
        for _ in 0..56 {
            let m = search[0] + (search[1] - search[0]) * 0.5;
            if m <= search[0] || m >= search[1] {
                break;
            }
            let v = residual(m)?;
            if side == 0 {
                if before(v) {
                    root[0] = m;
                    search[0] = m;
                } else {
                    search[1] = m;
                }
            } else if after(v) {
                root[1] = m;
                search[1] = m;
            } else {
                search[0] = m;
            }
        }
    }
    check(
        before(residual(root[0])?) && after(residual(root[1])?),
        "Axial stock isolation lost original endpoint signs",
    )?;
    let gap = if increasing {
        I::point(root[0])
    } else {
        I::point(1.).sub(I::point(root[1]))?
    };
    let magnitude = if increasing {
        derivative_lo
    } else {
        -derivative_hi
    };
    let lower = I::point(magnitude)
        .mul(gap)?
        .div(I::point(2.).mul(I::point(radial_upper))?)?
        .lo
        .max(0.);
    Ok(AxialStockIntersection {
        spec,
        flank,
        y,
        radius,
        cells,
        root,
        derivative: [derivative_lo, derivative_hi],
        radial_upper,
        cross_norm_lower: lower,
    })
}
