//! Finite physical-Y source domains, independent of angular chart coordinates.
use super::*;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct AxialDomain {
    pub t: [f64; 2],
    /// Original physical coordinate along the wheel Y axis, in model units.
    pub y: [f64; 2],
}

#[derive(Clone, Debug)]
pub struct AxialChartBounds {
    pub point: [[f64; 2]; 3],
    pub dt: [[f64; 2]; 3],
    pub dy: [[f64; 2]; 3],
    /// f^(6)/6! for local t and physical Y.
    pub sixth: [[[f64; 2]; 3]; 2],
    pub regularity_proven: bool,
}

/// Original generated root restricted by physical axial planes.
#[derive(Clone, Debug)]
pub struct AxialRootChart {
    angular: RootChart,
    y: [f64; 2],
    cross_norm_lower: f64,
}
impl AxialRootChart {
    pub fn specification(&self) -> Spec {
        self.angular.specification()
    }
    pub fn part(&self) -> RootPart {
        self.angular.part()
    }
    pub fn angular_source(&self) -> &RootChart {
        &self.angular
    }
    pub fn domain(&self) -> AxialDomain {
        AxialDomain {
            t: [0., 1.],
            y: self.y,
        }
    }
    pub fn cross_norm_lower(&self) -> f64 {
        self.cross_norm_lower
    }
    pub fn reverify(&self) -> Result<()> {
        let fresh = axial_root_chart(self.specification(), self.part(), self.y)?;
        check(
            fresh.angular.domain() == self.angular.domain()
                && fresh.angular.certificate() == self.angular.certificate()
                && fresh.cross_norm_lower == self.cross_norm_lower,
            "Original axial root domain or contact proof changed",
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
            "Axial root query must remain inside its original t/physical-Y domain",
        )
    }
    pub fn position_bounds(&self, d: AxialDomain) -> Result<[[f64; 2]; 3]> {
        self.validate_query(d)?;
        Ok(
            super::chart::axial_root_jets(self.specification(), self.part(), d)?
                .0
                .map(pair),
        )
    }
    pub fn enclose(&self, d: AxialDomain) -> Result<AxialChartBounds> {
        self.validate_query(d)?;
        let (p, dt, dy) = super::chart::axial_root_jets(self.specification(), self.part(), d)?;
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
                super::taylor::axial_root_sixth(self.specification(), self.part(), d, 0)?.map(pair),
                super::taylor::axial_root_sixth(self.specification(), self.part(), d, 1)?.map(pair),
            ],
            regularity_proven: self.cross_norm_lower > 0.
                || cross.iter().any(|x| x.lo > 0. || x.hi < 0.),
        })
    }
}

/// Uses beta=asin(Y/Rc); the Y bounds remain physical plane locations.
/// Original contact/rank/injectivity transfer through this monotone coordinate
/// map. This is an exact source reparameterization, not a projected/snapped cap.
pub fn axial_root_chart(spec: Spec, part: RootPart, y: [f64; 2]) -> Result<AxialRootChart> {
    let yi = iv(y)?;
    let p = worm::derive_parameters(spec.worm)?;
    let rc = iv(p.crest_radius)?;
    let beta = super::expression::asin_interval(yi.div(rc)?)?;
    let angular = root_chart(spec, part, pair(beta))?;
    let denominator = rc.mul(iv(angular.certificate().cosine)?)?;
    let lower = I::point(angular.certificate().cross_norm_lower)
        .div(denominator)?
        .lo
        .max(0.);
    Ok(AxialRootChart {
        angular,
        y,
        cross_norm_lower: lower,
    })
}
