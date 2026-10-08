//! Exact two-ended Hermite transition. Boundary ownership is retained as
//! section expressions; this is not a sewn Surface or closed Body.
use crate::{Result, check, interval_eval::Interval as I, surface_weight_profile::ProfileSection};
pub struct ProfileSectionBridge<'a, 'b, 'c, 'd> {
    sections: [Section<'a, 'b, 'c, 'd>; 2],
    scales: [f64; 2],
}
// Separate lifetimes permit independently owned source profiles.
enum Section<'a, 'b, 'c, 'd> {
    First(ProfileSection<'a, 'b>),
    Second(ProfileSection<'c, 'd>),
}
impl Section<'_, '_, '_, '_> {
    fn get(&self) -> &ProfileSection<'_, '_> {
        match self {
            Self::First(s) => s,
            Self::Second(s) => s,
        }
    }
}
impl<'a, 'b, 'c, 'd> ProfileSectionBridge<'a, 'b, 'c, 'd> {
    pub fn new(
        first: ProfileSection<'a, 'b>,
        second: ProfileSection<'c, 'd>,
        scales: [f64; 2],
    ) -> Result<Self> {
        check(
            first.range() == second.range() && scales.iter().all(|x| x.is_finite() && *x != 0.),
            "Require common original free parameters and finite nonzero tangent scales",
        )?;
        Ok(Self {
            sections: [Section::First(first), Section::Second(second)],
            scales,
        })
    }
    pub fn section(&self, high: bool) -> &ProfileSection<'_, '_> {
        self.sections[usize::from(high)].get()
    }
    fn validate(&self, free: [f64; 2], across: [f64; 2], budget: usize) -> Result<()> {
        self.section(false).rectangle(free)?;
        self.section(true).rectangle(free)?;
        check(
            budget <= 100000
                && across.iter().all(|x| x.is_finite())
                && across[0] >= 0.
                && across[1] <= 1.
                && across[0] <= across[1],
            "Bound Hermite bridge domain and work",
        )
    }
    fn data(
        &self,
        free: [f64; 2],
        budget: usize,
        jet: bool,
    ) -> Result<(Option<Vec<Vec<[[f64; 2]; 3]>>>, usize)> {
        let mut values = Vec::new();
        let mut spent = 0;
        for high in [false, true] {
            let s = self.section(high);
            let mut side = Vec::new();
            for query in 0..if jet { 4 } else { 2 } {
                if spent == budget {
                    return Ok((None, spent));
                }
                let (bounds, used) = match query {
                    0 => s.enclose(free, budget - spent)?,
                    1 => s.transverse_bounds(free, budget - spent)?,
                    2 => s.tangent_bounds(free, budget - spent)?,
                    _ => s
                        .profile()
                        .mixed_derivative_bounds(s.rectangle(free)?, budget - spent)?,
                };
                spent += used;
                let Some(bounds) = bounds else {
                    return Ok((None, spent));
                };
                side.push(bounds);
            }
            values.push(side);
        }
        Ok((Some(values), spent))
    }
    pub fn enclose(
        &self,
        free: [f64; 2],
        across: [f64; 2],
        budget: usize,
    ) -> Result<(Option<[[f64; 2]; 3]>, usize)> {
        self.validate(free, across, budget)?;
        if across == [0., 0.] {
            return self.section(false).enclose(free, budget);
        }
        if across == [1., 1.] {
            return self.section(true).enclose(free, budget);
        }
        let (data, spent) = self.data(free, budget, false)?;
        let Some(data) = data else {
            return Ok((None, spent));
        };
        let t = I::new(across[0], across[1])?;
        let q = I::point(1.).sub(t)?;
        let h = [
            q.mul(q)?.mul(I::point(1.).add(I::point(2.).mul(t)?)?)?,
            t.mul(q)?.mul(q)?.mul(I::point(self.scales[0]))?,
            t.mul(t)?.mul(I::point(3.).sub(I::point(2.).mul(t)?)?)?,
            I::point(-1.)
                .mul(t)?
                .mul(t)?
                .mul(q)?
                .mul(I::point(self.scales[1]))?,
        ];
        let mut bounds = [[0.; 2]; 3];
        for k in 0..3 {
            let mut value = I::point(0.);
            for (term, (side, slot)) in [(0, 0), (0, 1), (1, 0), (1, 1)].into_iter().enumerate() {
                value = value
                    .add(h[term].mul(I::new(data[side][slot][k][0], data[side][slot][k][1])?)?)?;
            }
            bounds[k] = [value.lo, value.hi];
        }
        Ok((Some(bounds), spent))
    }
    /// Both section boundaries agree identically in position and tangent plane.
    /// Nonzero normals are still required; no interior regularity follows.
    pub fn regular_join(
        &self,
        high: bool,
        free: [f64; 2],
        budget: usize,
    ) -> Result<(Option<bool>, usize)> {
        let (n, spent) = self.section(high).support_normals(free, budget)?;
        Ok((
            n.and_then(|n| n.iter().any(|r| r[0] > 0. || r[1] < 0.).then_some(true)),
            spent,
        ))
    }
    pub fn normal_bounds(
        &self,
        free: [f64; 2],
        across: [f64; 2],
        budget: usize,
    ) -> Result<(Option<[[f64; 2]; 3]>, usize)> {
        self.validate(free, across, budget)?;
        let (data, spent) = self.data(free, budget, true)?;
        let Some(data) = data else {
            return Ok((None, spent));
        };
        let t = I::new(across[0], across[1])?;
        let q = I::point(1.).sub(t)?;
        let h = [
            q.mul(q)?.mul(I::point(1.).add(I::point(2.).mul(t)?)?)?,
            t.mul(q)?.mul(q)?.mul(I::point(self.scales[0]))?,
            t.mul(t)?.mul(I::point(3.).sub(I::point(2.).mul(t)?)?)?,
            I::point(-1.)
                .mul(t)?
                .mul(t)?
                .mul(q)?
                .mul(I::point(self.scales[1]))?,
        ];
        let d = [
            I::point(-6.).mul(t)?.mul(q)?,
            q.mul(I::point(1.).sub(I::point(3.).mul(t)?)?)?
                .mul(I::point(self.scales[0]))?,
            I::point(6.).mul(t)?.mul(q)?,
            t.mul(I::point(3.).mul(t)?.sub(I::point(2.))?)?
                .mul(I::point(self.scales[1]))?,
        ];
        let mut free_d = [I::point(0.); 3];
        let mut across_d = free_d;
        for k in 0..3 {
            for (term, (side, slot)) in [(0, 0), (0, 1), (1, 0), (1, 1)].into_iter().enumerate() {
                across_d[k] = across_d[k]
                    .add(d[term].mul(I::new(data[side][slot][k][0], data[side][slot][k][1])?)?)?;
                free_d[k] = free_d[k].add(h[term].mul(I::new(
                    data[side][slot + 2][k][0],
                    data[side][slot + 2][k][1],
                )?)?)?;
            }
        }
        let mut bounds = [[0.; 2]; 3];
        for k in 0..3 {
            let i = (k + 1) % 3;
            let j = (k + 2) % 3;
            let n = free_d[i]
                .mul(across_d[j])?
                .sub(free_d[j].mul(across_d[i])?)?;
            bounds[k] = [n.lo, n.hi];
        }
        Ok((Some(bounds), spent))
    }
}
