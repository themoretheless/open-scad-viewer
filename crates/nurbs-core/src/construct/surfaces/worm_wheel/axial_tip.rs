//! Original stock-cylinder tooth tip between its two true generated flank ends.
use super::*;

#[derive(Clone, Debug)]
pub struct AxialTipChart {
    descending: AxialStockIntersection,
    ascending: AxialStockIntersection,
    proof_cells: usize,
    cross_product: [f64; 2],
    dot_product: [f64; 2],
    cross_norm_lower: f64,
}
fn same_spec(a: Spec, b: Spec) -> bool {
    let x = a.worm;
    let y = b.worm;
    a.teeth == b.teeth
        && a.start == b.start
        && a.axial_shift.to_bits() == b.axial_shift.to_bits()
        && x.hand == y.hand
        && x.starts == y.starts
        && x.turns == y.turns
        && [
            x.axial_module,
            x.diameter_quotient,
            x.pressure_angle,
            x.dedendum_factor,
            x.backlash,
            x.error_budget,
        ]
        .iter()
        .zip([
            y.axial_module,
            y.diameter_quotient,
            y.pressure_angle,
            y.dedendum_factor,
            y.backlash,
            y.error_budget,
        ])
        .all(|(a, b)| a.to_bits() == b.to_bits())
}
impl AxialTipChart {
    pub fn specification(&self) -> Spec {
        self.descending.specification()
    }
    pub fn radius(&self) -> f64 {
        self.descending.radius()
    }
    pub fn domain(&self) -> AxialDomain {
        self.descending.domain()
    }
    pub fn flank_sources(&self) -> [&AxialStockIntersection; 2] {
        [&self.descending, &self.ascending]
    }
    /// Entire original Y range: cross is negative (clockwise), dot positive.
    pub fn cross_product(&self) -> [f64; 2] {
        self.cross_product
    }
    pub fn dot_product(&self) -> [f64; 2] {
        self.dot_product
    }
    pub fn cross_norm_lower(&self) -> f64 {
        self.cross_norm_lower
    }
    pub fn reverify(&self) -> Result<()> {
        let fresh = axial_tip_chart(&self.descending, &self.ascending, self.proof_cells)?;
        check(
            fresh.cross_product == self.cross_product
                && fresh.dot_product == self.dot_product
                && fresh.cross_norm_lower == self.cross_norm_lower,
            "Original stock tip proof changed",
        )
    }
    pub(super) fn sample_y(&self, y: [f64; 2]) -> Result<Self> {
        self.validate_query(AxialDomain { t: [0., 1.], y })?;
        axial_tip_chart(
            &self.descending.restrict_y(y)?,
            &self.ascending.restrict_y(y)?,
            1,
        )
    }
    pub(super) fn validate_query(&self, d: AxialDomain) -> Result<()> {
        self.descending.validate_query(d)
    }
    fn roots(&self, y: [f64; 2]) -> Result<[AxialStockIntersection; 2]> {
        Ok([
            if y == self.domain().y {
                self.descending.clone()
            } else {
                self.descending.restrict_y(y)?
            },
            if y == self.domain().y {
                self.ascending.clone()
            } else {
                self.ascending.restrict_y(y)?
            },
        ])
    }
    pub fn position_bounds(&self, d: AxialDomain) -> Result<[[f64; 2]; 3]> {
        self.validate_query(d)?;
        let roots = self.roots(d.y)?;
        let a = roots[0].root_jets()?;
        let b = roots[1].root_jets()?;
        let endpoints = [
            roots[0].position_bounds(AxialDomain {
                t: [1., 1.],
                y: d.y,
            })?,
            roots[1].position_bounds(AxialDomain {
                t: [1., 1.],
                y: d.y,
            })?,
        ];
        Ok(super::chart::axial_tip_jets(
            self.specification(),
            self.radius(),
            d,
            [roots[0].u_bracket(), roots[1].u_bracket()],
            [a.1, b.1],
            endpoints,
        )?
        .0
        .map(pair))
    }
    pub fn enclose(&self, d: AxialDomain) -> Result<AxialChartBounds> {
        self.validate_query(d)?;
        let roots = self.roots(d.y)?;
        let a = roots[0].root_jets()?;
        let b = roots[1].root_jets()?;
        let brackets = [roots[0].u_bracket(), roots[1].u_bracket()];
        let endpoints = [
            roots[0].position_bounds(AxialDomain {
                t: [1., 1.],
                y: d.y,
            })?,
            roots[1].position_bounds(AxialDomain {
                t: [1., 1.],
                y: d.y,
            })?,
        ];
        let (p, dt, dy) = super::chart::axial_tip_jets(
            self.specification(),
            self.radius(),
            d,
            brackets,
            [a.1, b.1],
            endpoints,
        )?;
        Ok(AxialChartBounds {
            point: p.map(pair),
            dt: dt.map(pair),
            dy: dy.map(pair),
            sixth: [
                super::taylor::axial_tip_sixth(
                    self.specification(),
                    self.radius(),
                    d,
                    brackets,
                    [a.0, b.0],
                    endpoints,
                    0,
                )?
                .map(pair),
                super::taylor::axial_tip_sixth(
                    self.specification(),
                    self.radius(),
                    d,
                    brackets,
                    [a.0, b.0],
                    endpoints,
                    1,
                )?
                .map(pair),
            ],
            regularity_proven: self.cross_norm_lower > 0.,
        })
    }
}

/// Both original stock ends lie on the same radius. Their positive dot product
/// and negative oriented cross product prove a unique, clockwise short circle
/// arc, not a straight loft. The source is R*((1-t)A+tB)/|(1-t)A+tB| in XZ,
/// with physical Y unchanged. Derivative rank and source injectivity follow
/// from strict angular progress and the exact Y coordinate.
pub fn axial_tip_chart(
    descending: &AxialStockIntersection,
    ascending: &AxialStockIntersection,
    proof_cells: usize,
) -> Result<AxialTipChart> {
    check(
        (1..=4096).contains(&proof_cells)
            && descending.flank() == Flank::Descending
            && ascending.flank() == Flank::Ascending
            && same_spec(descending.specification(), ascending.specification())
            && descending.radius().to_bits() == ascending.radius().to_bits()
            && descending.domain() == ascending.domain(),
        "Stock tip needs matching original descending/ascending stock sources and bounded proof work",
    )?;
    descending.reverify()?;
    ascending.reverify()?;
    let y = descending.domain().y;
    let mut cross = None::<I>;
    let mut dot = None::<I>;
    let mut left = y[0];
    for i in 1..=proof_cells {
        let right = if i == proof_cells {
            y[1]
        } else {
            y[0] + (y[1] - y[0]) * (i as f64 / proof_cells as f64)
        };
        check(left <= right, "Stock tip proof lost physical-Y coverage")?;
        let d = AxialDomain {
            t: [1., 1.],
            y: [left, right],
        };
        let a = descending.position_bounds(d)?;
        let b = ascending.position_bounds(d)?;
        let c = iv(a[0])?.mul(iv(b[2])?)?.sub(iv(a[2])?.mul(iv(b[0])?)?)?;
        let q = iv(a[0])?.mul(iv(b[0])?)?.add(iv(a[2])?.mul(iv(b[2])?)?)?;
        check(
            c.hi < 0. && q.lo > 0.,
            "Stock tip endpoints do not prove a clockwise short material arc",
        )?;
        cross = Some(match cross {
            None => c,
            Some(old) => I::new(old.lo.min(c.lo), old.hi.max(c.hi))?,
        });
        dot = Some(match dot {
            None => q,
            Some(old) => I::new(old.lo.min(q.lo), old.hi.max(q.hi))?,
        });
        left = right;
    }
    let cross = cross.unwrap();
    let lower = I::point(-cross.hi)
        .div(I::point(descending.radius()))?
        .lo
        .max(0.);
    Ok(AxialTipChart {
        descending: descending.clone(),
        ascending: ascending.clone(),
        proof_cells,
        cross_product: pair(cross),
        dot_product: pair(dot.unwrap()),
        cross_norm_lower: lower,
    })
}
