//! Exact two-ended Hermite transition. Boundary ownership is retained as
//! section expressions; this is not a sewn Surface or closed Body.
use crate::{Result, check, interval_eval::Interval as I, surface_weight_profile::ProfileSection};
fn projected_principal_minors_positive(j: [[I; 2]; 2]) -> Result<bool> {
    let off = j[0][1].add(j[1][0])?.mul(I::point(0.5))?;
    let symmetric = j[0][0].mul(j[1][1])?.sub(off.mul(off)?)?;
    let determinant = j[0][0].mul(j[1][1])?.sub(j[0][1].mul(j[1][0])?)?;
    Ok(j[0][0].lo > 0. && j[1][1].lo > 0. && (determinant.lo > 0. || symmetric.lo > 0.))
}
// h01(t)=t^2(3-2t) increases on [0,1] since h01'=6t(1-t).
// Point interval evaluation encloses both endpoint values; analytic monotonicity
// encloses the whole interval without treating samples as authority.
fn anchored_position_range(across:[f64;2],a:I,b:I,tangent:I,direct:I)->Result<I> {
    check(across.iter().all(|v|v.is_finite()) && across[0]>=0. && across[0]<=across[1] && across[1]<=1., "Keep Hermite blend in [0,1]")?;
    let value=|x:f64|->Result<I> {let t=I::point(x);t.mul(t)?.mul(I::point(3.).sub(I::point(2.).mul(t)?)?)};
    let left=value(across[0])?;let right=value(across[1])?;
    let w=I::new(left.lo.max(0.),right.hi.min(1.))?;
    let first=a.add(w.mul(b.sub(a)?)?)?.add(tangent)?;
    let second=b.add(I::point(1.).sub(w)?.mul(a.sub(b)?)?)?.add(tangent)?;
    // All three forms are the same exact Hermite expression. Their outward
    // enclosures may be intersected; no source/profile substitution is made.
    I::new(direct.lo.max(first.lo).max(second.lo),direct.hi.min(first.hi).min(second.hi))
}
#[path="profile_bridge_chord_coverage.rs"]
mod chord_coverage;
pub use chord_coverage::ProfileBridgeChordCoverage;
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
    pub fn rail(&self, high: bool) -> ProfileBridgeRail<'_, 'a, 'b, 'c, 'd> {
        ProfileBridgeRail {
            bridge: self,
            free: self.section(false).range()[usize::from(high)],
        }
    }
    /// Attach source-natural rails only when both sections and endpoint jets
    /// have the same exact affine support coordinate. Rounded scales refuse.
    pub fn constrain_original_rails(
        &self,
        max_controls: usize,
    ) -> Result<(Option<RailConstrainedBridge<'_, 'a, 'b, 'c, 'd>>, usize)> {
        check(max_controls <= 1000000, "Bound original rail control work")?;
        let a = self.section(false);
        let b = self.section(true);
        if !std::ptr::eq(a.profile(), b.profile()) || a.axis() != b.axis() {
            return Ok((None, 0));
        }
        let surface = a.profile().source();
        for (knots, degree, count) in [
            (
                &surface.knots_u,
                surface.degree_u,
                surface.control_points.len(),
            ),
            (
                &surface.knots_v,
                surface.degree_v,
                surface.control_points[0].len(),
            ),
        ] {
            if knots.iter().any(|&k| {
                k > knots[degree]
                    && k < knots[count]
                    && knots.iter().filter(|&&x| x == k).count() >= degree
            }) {
                return Ok((None, 0));
            }
        }
        let x = a.parameter();
        let y = b.parameter();
        let width = y - x;
        // Sterbenz guarantees this binary subtraction is exact. Zero is exact too.
        let exact = x == 0.
            || y == 0.
            || (x.signum() == y.signum() && x.abs() * 0.5 <= y.abs() && y.abs() <= x.abs() * 2.);
        if !exact || width == 0. || self.scales.iter().any(|s| s.to_bits() != width.to_bits()) {
            return Ok((None, 0));
        }
        let mut spent = 0;
        for high in [false, true] {
            let (boundary, used) =
                a.profile()
                    .preserved_boundary(1 - a.axis(), high, max_controls - spent)?;
            spent += used;
            if boundary.is_none() {
                return Ok((None, spent));
            }
        }
        Ok((Some(RailConstrainedBridge { bridge: self }), spent))
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
        self.enclose_cached(free, across, budget, None)
    }
    fn enclose_cached(
        &self, free: [f64; 2], across: [f64; 2], budget: usize,
        cache: Option<&SideRemainderCache>,
    ) -> Result<(Option<[[f64; 2]; 3]>, usize)> {
        self.validate(free, across, budget)?;
        if across == [0., 0.] {
            return self.section(false).enclose(free, budget);
        }
        if across == [1., 1.] {
            return self.section(true).enclose(free, budget);
        }
        // A complete derivative jet contains the same position/transverse
        // hulls. Reuse only this bridge and exact interval; zero budget must
        // still refuse, even when a previous query populated the cache.
        let key=(self as *const Self as usize,free.map(f64::to_bits));
        let cached=if budget>0 {cache.and_then(|c|c.bridge_data.get(&key))} else {None};
        let (data, spent) = match cached {
            Some(data)=>(Some(data.clone()),0),
            None=>self.data(free,budget,false)?,
        };
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
            let a=I::new(data[0][0][k][0],data[0][0][k][1])?;
            let b=I::new(data[1][0][k][0],data[1][0][k][1])?;
            let tangent=h[1].mul(I::new(data[0][1][k][0],data[0][1][k][1])?)?
                .add(h[3].mul(I::new(data[1][1][k][0],data[1][1][k][1])?)?)?;
            let value=anchored_position_range(across,a,b,tangent,value)?;
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
    pub fn derivative_bounds(
        &self,
        free: [f64; 2],
        across: [f64; 2],
        budget: usize,
    ) -> Result<(Option<[[[f64; 2]; 3]; 2]>, usize)> {
        self.derivative_bounds_impl(free, across, budget, false)
    }
    fn derivative_bounds_impl(
        &self,
        free: [f64; 2],
        across: [f64; 2],
        budget: usize,
        common_difference: bool,
    ) -> Result<(Option<[[[f64; 2]; 3]; 2]>, usize)> {
        self.derivative_bounds_cached(free, across, budget, common_difference, None)
    }
    fn derivative_bounds_cached(
        &self, free: [f64; 2], across: [f64; 2], budget: usize,
        common_difference: bool, cache: Option<&mut SideRemainderCache>,
    ) -> Result<(Option<[[[f64; 2]; 3]; 2]>, usize)> {
        self.validate(free, across, budget)?;
        // Section data is independent of the across parameter. Only complete
        // jets of this immutable bridge and exact free interval are reusable.
        let key=(self as *const Self as usize,free.map(f64::to_bits));
        let (data, mut spent) = if let Some(cache)=cache {
            if let Some(data)=cache.bridge_data.get(&key) {
                (Some(data.clone()),0)
            } else {
                let result=self.data(free,budget,true)?;
                if let Some(data)=&result.0 {
                    if cache.bridge_data.len()<100000 {cache.bridge_data.insert(key,data.clone());}
                }
                result
            }
        } else {self.data(free,budget,true)?};
        let Some(data) = data else {
            return Ok((None, spent));
        };
        let common = if common_difference {
            let a = self.section(false);
            let b = self.section(true);
            check(
                std::ptr::eq(a.profile(), b.profile()) && a.axis() == b.axis(),
                "Require common source difference ownership",
            )?;
            if spent == budget {
                return Ok((None, spent));
            }
            let (difference, used) = a.profile().coordinate_difference_bounds(
                a.axis(),
                [a.parameter(), b.parameter()],
                free,
                budget - spent,
            )?;
            spent += used;
            let Some(difference) = difference else {
                return Ok((None, spent));
            };
            Some(difference)
        } else {
            None
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
                if slot == 1 {
                    across_d[k] = across_d[k].add(
                        d[term].mul(I::new(data[side][slot][k][0], data[side][slot][k][1])?)?,
                    )?;
                }
                free_d[k] = free_d[k].add(h[term].mul(I::new(
                    data[side][slot + 2][k][0],
                    data[side][slot + 2][k][1],
                )?)?)?;
            }
        }
        for k in 0..3 {
            let mut difference = I::new(data[1][0][k][0], data[1][0][k][1])?
                .sub(I::new(data[0][0][k][0], data[0][0][k][1])?)?;
            if let Some(common) = common {
                difference = difference.intersect(common[k][0], common[k][1])?;
            }
            across_d[k] = across_d[k].add(d[2].mul(difference)?)?;
            // The transverse derivative is a quadratic Bezier polynomial.
            // Restrict its three controls before taking a hull, preserving
            // cancellation between Hermite basis terms over wide across ranges.
            let first=I::new(data[0][1][k][0],data[0][1][k][1])?.mul(I::point(self.scales[0]))?;
            let last=I::new(data[1][1][k][0],data[1][1][k][1])?.mul(I::point(self.scales[1]))?;
            let middle=I::point(3.).mul(difference)?.sub(first)?.sub(last)?;
            let blend=|a:I,b:I,t:I| -> Result<I> {
                I::point(1.).sub(t)?.mul(a)?.add(t.mul(b)?)
            };
            let blossom=|a:f64,b:f64| -> Result<I> {
                let a=I::point(a);let b=I::point(b);
                blend(blend(first,middle,a)?,blend(middle,last,a)?,b)
            };
            let controls=[blossom(across[0],across[0])?,
                blossom(across[0],across[1])?,blossom(across[1],across[1])?];
            let lo=controls.iter().map(|c|c.lo).fold(f64::INFINITY,f64::min);
            let hi=controls.iter().map(|c|c.hi).fold(f64::NEG_INFINITY,f64::max);
            across_d[k]=across_d[k].intersect(lo,hi)?;
            // The free derivative is the corresponding cubic Hermite field.
            // Restrict its Bezier controls independently of the across basis.
            let f0=I::new(data[0][2][k][0],data[0][2][k][1])?;
            let f1=I::new(data[1][2][k][0],data[1][2][k][1])?;
            let g0=I::new(data[0][3][k][0],data[0][3][k][1])?.mul(I::point(self.scales[0]))?;
            let g1=I::new(data[1][3][k][0],data[1][3][k][1])?.mul(I::point(self.scales[1]))?;
            let cubic=[f0,f0.add(g0.div(I::point(3.))?)?,f1.sub(g1.div(I::point(3.))?)?,f1];
            let blossom3=|parameters:[f64;3]| -> Result<I> {
                let mut work=cubic;
                for (level,t) in parameters.into_iter().enumerate() {
                    for i in 0..3-level {work[i]=blend(work[i],work[i+1],I::point(t))?;}
                }
                Ok(work[0])
            };
            let [a,b]=across;
            let controls=[blossom3([a,a,a])?,blossom3([a,a,b])?,
                blossom3([a,b,b])?,blossom3([b,b,b])?];
            let lo=controls.iter().map(|c|c.lo).fold(f64::INFINITY,f64::min);
            let hi=controls.iter().map(|c|c.hi).fold(f64::NEG_INFINITY,f64::max);
            free_d[k]=free_d[k].intersect(lo,hi)?;
        }
        Ok((
            Some([free_d.map(|r| [r.lo, r.hi]), across_d.map(|r| [r.lo, r.hi])]),
            spent,
        ))
    }
    /// Mixed derivative with respect to the original free parameter and
    /// normalized bridge coordinate. Uses original expression jets throughout.
    pub fn mixed_derivative_bounds(
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
        let d0 = q
            .mul(I::point(1.).sub(I::point(3.).mul(t)?)?)?
            .mul(I::point(self.scales[0]))?;
        let d1 = t
            .mul(I::point(3.).mul(t)?.sub(I::point(2.))?)?
            .mul(I::point(self.scales[1]))?;
        let position = I::point(6.).mul(t)?.mul(q)?;
        let mut bounds = [[0.; 2]; 3];
        for k in 0..3 {
            let v =
                |side: usize, slot: usize| I::new(data[side][slot][k][0], data[side][slot][k][1]);
            let value = position
                .mul(v(1, 2)?.sub(v(0, 2)?)?)?
                .add(d0.mul(v(0, 3)?)?)?
                .add(d1.mul(v(1, 3)?)?)?;
            // Differentiate the cubic free field in Bezier form, retaining
            // cancellation between all transverse Hermite basis terms.
            let first=v(0,3)?.mul(I::point(self.scales[0]))?;
            let last=v(1,3)?.mul(I::point(self.scales[1]))?;
            let middle=I::point(3.).mul(v(1,2)?.sub(v(0,2)?)?)?.sub(first)?.sub(last)?;
            let blend=|a:I,b:I,t:I| -> Result<I> {I::point(1.).sub(t)?.mul(a)?.add(t.mul(b)?)};
            let blossom=|a:f64,b:f64| -> Result<I> {
                let a=I::point(a);let b=I::point(b);
                blend(blend(first,middle,a)?,blend(middle,last,a)?,b)
            };
            let controls=[blossom(across[0],across[0])?,blossom(across[0],across[1])?,
                blossom(across[1],across[1])?];
            let lo=controls.iter().map(|c|c.lo).fold(f64::INFINITY,f64::min);
            let hi=controls.iter().map(|c|c.hi).fold(f64::NEG_INFINITY,f64::max);
            let value=value.intersect(lo,hi)?;
            bounds[k] = [value.lo, value.hi];
        }
        Ok((Some(bounds), spent))
    }
    pub fn normal_bounds(
        &self,
        free: [f64; 2],
        across: [f64; 2],
        budget: usize,
    ) -> Result<(Option<[[f64; 2]; 3]>, usize)> {
        let (jet, spent) = self.derivative_bounds(free, across, budget)?;
        let Some([a, b]) = jet else {
            return Ok((None, spent));
        };
        let mut bounds = [[0.; 2]; 3];
        for k in 0..3 {
            let i = (k + 1) % 3;
            let j = (k + 2) % 3;
            let n = I::new(a[i][0], a[i][1])?
                .mul(I::new(b[j][0], b[j][1])?)?
                .sub(I::new(a[j][0], a[j][1])?.mul(I::new(b[i][0], b[i][1])?)?)?;
            bounds[k] = [n.lo, n.hi];
        }
        Ok((Some(bounds), spent))
    }
    fn chord_continuous(&self)->bool {
        // Same conservative C1 knot policy as bridge injectivity.
        for high in [false,true] {
            let s=self.section(high).profile().source();
            for (knots,degree,count) in [(&s.knots_u,s.degree_u,s.control_points.len()),
                (&s.knots_v,s.degree_v,s.control_points[0].len())] {
                if degree==0 || knots.iter().any(|&k|k>knots[degree] && k<knots[count]
                    && knots.iter().filter(|&&x|x==k).count()>=degree) {return false;}
            }
        }
        true
    }
    /// Exclude normal-aligned chords between two parameter rectangles on
    /// this immutable bridge. Both coordinate paths cover the same chord;
    /// only complete derivative/normal enclosures can issue an exclusion.
    pub fn inspect_self_chord_pair(
        &self,pair:[[[f64;2];2];2],sine:f64,max_spans:usize,
    )->Result<(Option<bool>,usize)> {
        check(pair.iter().flatten().all(|d|d.iter().all(|x|x.is_finite()) && d[0]<d[1]),
            "Positive finite bridge endpoint rectangles")?;
        let union=std::array::from_fn(|k|[pair[0][k][0].min(pair[1][k][0]),pair[0][k][1].max(pair[1][k][1])]);
        self.inspect_self_chords(union,sine,0)?;
        check(max_spans<=100000,"Bound bridge endpoint chord work")?;
        if max_spans==0 || !self.chord_continuous() {return Ok((None,0));}
        let (jet,mut spent)=self.derivative_bounds(union[0],union[1],max_spans)?;
        let Some(jet)=jet else {return Ok((None,spent));};
        let matrix=std::array::from_fn(|k|std::array::from_fn(|axis|jet[axis][k]));
        let ranked=crate::surface_self_chord::inspect_jacobian_rank_bound(matrix)?;
        let mut normals=[[[0.;2];3];2];
        for (i,rect) in pair.into_iter().enumerate() {
            if spent==max_spans {return Ok((None,spent));}
            let (normal,used)=self.normal_bounds(rect[0],rect[1],max_spans-spent)?;spent+=used;
            let Some(normal)=normal else {return Ok((None,spent));};normals[i]=normal;
            if crate::surface_self_chord::inspect_jacobian_endpoint_pair_chords(matrix,pair,normal,sine)?
                || (ranked && crate::surface_self_chord::inspect_jacobian_endpoint_chords(matrix,normal,sine)?.0) {
                return Ok((Some(true),spent));
            }
        }
        let mut delta=[I::point(0.);2];
        for axis in 0..2 {delta[axis]=I::new(pair[1][axis][0],pair[1][axis][1])?
            .sub(I::new(pair[0][axis][0],pair[0][axis][1])?)?;}
        let mut chord=[[f64::NEG_INFINITY,f64::INFINITY];3];
        for reverse in [false,true] {
            let strips=if reverse {[[union[0],pair[1][1]],[pair[0][0],union[1]]]}
                else {[[union[0],pair[0][1]],[pair[1][0],union[1]]]};
            let mut columns=matrix;
            for axis in 0..2 {
                if spent==max_spans {return Ok((None,spent));}
                let (jet,used)=self.derivative_bounds(strips[axis][0],strips[axis][1],max_spans-spent)?;spent+=used;
                let Some(jet)=jet else {return Ok((None,spent));};
                for k in 0..3 {columns[k][axis]=jet[axis][k];}
            }
            for k in 0..3 {
                let value=I::new(columns[k][0][0],columns[k][0][1])?.mul(delta[0])?
                    .add(I::new(columns[k][1][0],columns[k][1][1])?.mul(delta[1])?)?;
                let overlap=I::new(chord[k][0].max(value.lo),chord[k][1].min(value.hi))?;
                chord[k]=[overlap.lo,overlap.hi];
            }
            for normal in normals {
                if crate::normal_alignment::inspect_pair_bounds([normal,chord],sine)?.aligned==Some(false) {
                    return Ok((Some(true),spent));
                }
            }
        }
        Ok((Some(false),spent))
    }
    /// Sufficient whole-rectangle exclusion of normal-aligned nonzero chords
    /// on this exact Hermite expression, including independently owned profiles.
    /// The immutable sections own all derivative bounds. No rail matching,
    /// radius, trim, shell or solid admission follows from this diagnostic.
    pub fn inspect_self_chords(
        &self, domain:[[f64;2];2], max_sine_squared:f64, max_spans:usize,
    )->Result<(Option<bool>,usize)> {
        self.validate(domain[0],domain[1],max_spans)?;
        check(max_sine_squared.is_finite() && (0. ..1.).contains(&max_sine_squared)
            && domain.iter().all(|d|d[0]<d[1]),"Bound bridge chord domain and angular policy")?;
        if !self.chord_continuous() {return Ok((None,0));}
        if max_spans==0 {return Ok((None,0));}
        let (jet,spent)=self.derivative_bounds(domain[0],domain[1],max_spans)?;
        let Some(jet)=jet else {return Ok((None,spent));};
        let matrix=std::array::from_fn(|k|std::array::from_fn(|axis|jet[axis][k]));
        Ok((Some(crate::surface_self_chord::inspect_jacobian_self_chords(matrix,max_sine_squared)?.0),spent))
    }
    /// Gale-Nikaido (1965), Theorem 4: a differentiable projected map on a
    /// closed rectangle is injective when its Jacobian is a P-matrix everywhere.
    /// In 2D all principal minors are j00, j11 and det(J). Complete interval
    /// coverage proves their positivity; symmetric positivity is also sufficient.
    /// https://fig.if.usp.br/~marchett/fluidos/jacobian-matrix-global-univalence-mapping_gale-nikaido.pdf
    pub fn inspect_injectivity(
        &self,
        free: [f64; 2],
        across: [f64; 2],
        projection: [[f64; 3]; 2],
        max_cells: usize,
        max_spans: usize,
    ) -> Result<crate::surface_weight_profile::TransitionRegularityReport> {
        self.validate(free, across, max_spans)?;
        check(
            max_cells <= 100000 && projection.iter().flatten().all(|x| x.is_finite()),
            "Bound injectivity work and projection",
        )?;
        let mut out = crate::surface_weight_profile::TransitionRegularityReport {
            regular: Vec::new(),
            pending: Vec::new(),
            cells: 0,
            spans: 0,
            complete_regular: false,
            reason: "section-bridge-injectivity-work-limit",
        };
        // The transverse field must be single-valued and piecewise C1. Refuse
        // internal knots whose multiplicity would allow first-derivative jumps.
        for high in [false, true] {
            let surface = self.section(high).profile().source();
            for (knots, degree, count) in [
                (
                    &surface.knots_u,
                    surface.degree_u,
                    surface.control_points.len(),
                ),
                (
                    &surface.knots_v,
                    surface.degree_v,
                    surface.control_points[0].len(),
                ),
            ] {
                if knots.iter().any(|&k| {
                    k > knots[degree]
                        && k < knots[count]
                        && knots.iter().filter(|&&x| x == k).count() >= degree
                }) {
                    out.pending.push([free, across]);
                    out.reason = "section-bridge-injectivity-continuity-unproven";
                    return Ok(out);
                }
            }
        }
        let mut queue = std::collections::VecDeque::from([[free, across]]);
        while let Some(rect) = queue.pop_front() {
            if out.cells == max_cells || out.spans == max_spans {
                out.pending.push(rect);
                out.pending.extend(queue);
                return Ok(out);
            }
            out.cells += 1;
            let (jet, spent) = self.derivative_bounds(rect[0], rect[1], max_spans - out.spans)?;
            out.spans += spent;
            let Some(jet) = jet else {
                out.pending.push(rect);
                out.pending.extend(queue);
                return Ok(out);
            };
            let mut j = [[I::point(0.); 2]; 2];
            for row in 0..2 {
                for axis in 0..2 {
                    for k in 0..3 {
                        j[row][axis] = j[row][axis].add(
                            I::point(projection[row][k])
                                .mul(I::new(jet[axis][k][0], jet[axis][k][1])?)?,
                        )?;
                    }
                }
            }
            if projected_principal_minors_positive(j)? {
                out.regular.push(rect);
                continue;
            }
            let range = self.section(false).range();
            let axis = usize::from(
                (rect[0][1] - rect[0][0]) / (range[1] - range[0]) < rect[1][1] - rect[1][0],
            );
            let mid = rect[axis][0] * 0.5 + rect[axis][1] * 0.5;
            if mid <= rect[axis][0] || mid >= rect[axis][1] {
                out.pending.push(rect);
                continue;
            }
            let mut left = rect;
            left[axis][1] = mid;
            let mut right = rect;
            right[axis][0] = mid;
            queue.push_back(left);
            queue.push_back(right);
        }
        out.complete_regular = out.pending.is_empty();
        out.reason = if out.complete_regular {
            "section-bridge-injectivity-qualified"
        } else {
            "section-bridge-injectivity-resolution-limit"
        };
        Ok(out)
    }
    /// Complete adaptive cover. Regularity is proved only by nonzero full-cell
    /// normal bounds; all unproved rectangles survive budget exhaustion.
    pub fn inspect_regularity(
        &self,
        free: [f64; 2],
        across: [f64; 2],
        max_cells: usize,
        max_spans: usize,
    ) -> Result<crate::surface_weight_profile::TransitionRegularityReport> {
        self.validate(free, across, max_spans)?;
        check(max_cells <= 100000, "Bound bridge cell work")?;
        let mut out = crate::surface_weight_profile::TransitionRegularityReport {
            regular: Vec::new(),
            pending: Vec::new(),
            cells: 0,
            spans: 0,
            complete_regular: false,
            reason: "section-bridge-regularity-work-limit",
        };
        let mut queue = std::collections::VecDeque::from([[free, across]]);
        while let Some(rect) = queue.pop_front() {
            if out.cells == max_cells || out.spans == max_spans {
                out.pending.push(rect);
                out.pending.extend(queue);
                return Ok(out);
            }
            out.cells += 1;
            let (bounds, spent) = self.normal_bounds(rect[0], rect[1], max_spans - out.spans)?;
            out.spans += spent;
            let Some(bounds) = bounds else {
                out.pending.push(rect);
                out.pending.extend(queue);
                return Ok(out);
            };
            if bounds.iter().any(|r| r[0] > 0. || r[1] < 0.) {
                out.regular.push(rect);
                continue;
            }
            let range = self.section(false).range();
            let axis = usize::from(
                (rect[0][1] - rect[0][0]) / (range[1] - range[0]) < rect[1][1] - rect[1][0],
            );
            let mid = rect[axis][0] * 0.5 + rect[axis][1] * 0.5;
            if mid <= rect[axis][0] || mid >= rect[axis][1] {
                out.pending.push(rect);
                continue;
            }
            let mut left = rect;
            left[axis][1] = mid;
            let mut right = rect;
            right[axis][0] = mid;
            queue.push_back(left);
            queue.push_back(right);
        }
        out.complete_regular = out.pending.is_empty();
        out.reason = if out.complete_regular {
            "section-bridge-regularity-qualified"
        } else {
            "section-bridge-regularity-resolution-limit"
        };
        Ok(out)
    }
}

/// Exact side-rail expression, retaining both incident section definitions.
pub struct ProfileBridgeRail<'r, 'a, 'b, 'c, 'd> {
    bridge: &'r ProfileSectionBridge<'a, 'b, 'c, 'd>,
    free: f64,
}
impl ProfileBridgeRail<'_, '_, '_, '_, '_> {
    pub fn free_parameter(&self) -> f64 {
        self.free
    }
    pub fn enclose(
        &self,
        range: [f64; 2],
        budget: usize,
    ) -> Result<(Option<[[f64; 2]; 3]>, usize)> {
        self.bridge.enclose([self.free; 2], range, budget)
    }
    pub fn tangent_bounds(
        &self,
        range: [f64; 2],
        budget: usize,
    ) -> Result<(Option<[[f64; 2]; 3]>, usize)> {
        let (jet, spent) = self
            .bridge
            .derivative_bounds([self.free; 2], range, budget)?;
        Ok((jet.map(|j| j[1]), spent))
    }
}
// Only complete source fourth-derivative hulls. The owning chord workspace
// retains both immutable profiles; profile identity and the exact domain/order
// prevent reuse across different rails or source geometries.
#[derive(Default)]
pub(crate) struct SideRemainderCache {
    bridge_data: std::collections::HashMap<(usize,[u64;2]),Vec<Vec<[[f64;2];3]>>>,
    rail_positions: std::collections::HashMap<(usize,bool,[u64;2],usize),[[f64;2];3]>,
    entries: std::collections::HashMap<(usize, [[u64; 2]; 2], [usize; 2]), [[f64; 2]; 3]>,
}
/// Position correction anchored to original natural support rails. It does
/// not inherit regularity/injectivity from the uncorrected Hermite bridge.
pub struct RailConstrainedBridge<'r, 'a, 'b, 'c, 'd> {
    bridge: &'r ProfileSectionBridge<'a, 'b, 'c, 'd>,
}
impl RailConstrainedBridge<'_, '_, '_, '_, '_> {
    /// Original section identity used to bind B-rep carrier ownership.
    pub fn section(&self, high: bool) -> &ProfileSection<'_, '_> {
        self.bridge.section(high)
    }

    fn source_rail_domain(&self, high: bool, across: [f64; 2]) -> Result<[[f64; 2]; 2]> {
        let a = self.bridge.section(false);
        let b = self.bridge.section(true);
        let t = I::new(across[0], across[1])?;
        let coordinate = I::point(a.parameter())
            .add(t.mul(I::point(b.parameter()).sub(I::point(a.parameter()))?)?)?;
        let mut domain = [[0.; 2]; 2];
        domain[a.axis()] = [coordinate.lo, coordinate.hi];
        domain[1 - a.axis()] = [a.range()[usize::from(high)]; 2];
        let surface = a.profile().source();
        let axis = a.axis();
        let chart = if axis == 0 {
            [
                surface.knots_u[surface.degree_u],
                surface.knots_u[surface.control_points.len()],
            ]
        } else {
            [
                surface.knots_v[surface.degree_v],
                surface.knots_v[surface.control_points[0].len()],
            ]
        };
        domain[axis][0] = domain[axis][0].max(chart[0]);
        domain[axis][1] = domain[axis][1].min(chart[1]);
        Ok(domain)
    }
    fn source_rail(
        &self,
        high: bool,
        across: [f64; 2],
        budget: usize,
    ) -> Result<(Option<[[f64; 2]; 3]>, usize)> {
        self.bridge
            .section(false)
            .profile()
            .enclose(self.source_rail_domain(high, across)?, budget)
    }
    /// Position enclosure for the full side Hermite correction. This is a
    /// distinct expression from `enclose`: its normals and regularity must be
    /// qualified separately before assembly.
    pub fn enclose_with_transverse_correction(
        &self,
        free: [f64; 2],
        across: [f64; 2],
        budget: usize,
    ) -> Result<(Option<[[f64; 2]; 3]>, usize)> {
        self.enclose_with_transverse_correction_cached(free,across,budget,None)
    }
    pub(crate) fn enclose_with_transverse_correction_cached(
        &self, free:[f64;2], across:[f64;2], budget:usize,
        cache:Option<&SideRemainderCache>,
    ) -> Result<(Option<[[f64;2];3]>,usize)> {
        self.bridge.validate(free, across, budget)?;
        let range = self.bridge.section(false).range();
        if across == [0., 0.] || across == [1., 1.] {
            return self.bridge.enclose(free, across, budget);
        }
        for high in [false, true] {
            if free == [range[usize::from(high)]; 2] {
                return self.source_rail(high, across, budget);
            }
        }
        let (base, mut spent) = self.bridge.enclose_cached(free, across, budget, cache)?;
        let Some(base) = base else {
            return Ok((None, spent));
        };
        let width = I::point(range[1]).sub(I::point(range[0]))?;
        let t = I::new(free[0], free[1])?
            .sub(I::point(range[0]))?
            .div(width)?;
        let q = I::point(1.).sub(t)?;
        let position_weights = [
            q.mul(q)?.mul(I::point(1.).add(I::point(2.).mul(t)?)?)?,
            t.mul(t)?.mul(I::point(3.).sub(I::point(2.).mul(t)?)?)?,
        ];
        let tangent_weights = [
            t.mul(q)?.mul(q)?.mul(width)?,
            I::point(-1.).mul(t)?.mul(t)?.mul(q)?.mul(width)?,
        ];
        let mut result = [I::point(0.); 3];
        for k in 0..3 {
            result[k] = I::new(base[k][0], base[k][1])?;
        }
        for high in [false, true] {
            let section = self.bridge.section(false);
            let mut values = Vec::new();
            for query in 0..4 {
                if spent == budget {
                    return Ok((None, spent));
                }
                let rail_key=(self.bridge as *const _ as usize,high,across.map(f64::to_bits),query);
                let cached=if query<2 {cache.and_then(|c|c.rail_positions.get(&rail_key))} else {None};
                let (value, used) = if let Some(value)=cached {(Some(*value),0)} else {match query {
                    0 => self.source_rail(high, across, budget - spent)?,
                    1 => self.bridge.rail(high).enclose(across, budget - spent)?,
                    2 => {
                        let (jet, used) = section.profile().derivative_bounds(
                            self.source_rail_domain(high, across)?,
                            budget - spent,
                        )?;
                        (
                            jet.map(|j| j.map(|component| component[1 - section.axis()])),
                            used,
                        )
                    }
                    _ => {
                        let (jet, used) = self.bridge.derivative_bounds(
                            [range[usize::from(high)]; 2],
                            across,
                            budget - spent,
                        )?;
                        (jet.map(|j| j[0]), used)
                    }
                }};
                spent += used;
                let Some(value) = value else {
                    return Ok((None, spent));
                };
                values.push(value);
            }
            for k in 0..3 {
                let v = |i: usize| I::new(values[i][k][0], values[i][k][1]);
                result[k] = result[k]
                    .add(position_weights[usize::from(high)].mul(v(0)?.sub(v(1)?)?)?)?
                    .add(tangent_weights[usize::from(high)].mul(v(2)?.sub(v(3)?)?)?)?;
            }
        }
        Ok((Some(result.map(|v| [v.lo, v.hi])), spent))
    }
    // Taylor polynomial terms through degree three cancel identically in
    // cubic Hermite interpolation. Bound only the value/derivative remainders.
    fn side_remainders(
        &self, high: bool, across: [f64; 2], budget: usize,
    ) -> Result<(Option<[[[f64; 2]; 3]; 4]>, usize)> {
        self.side_remainders_cached(high, across, budget, &mut SideRemainderCache::default())
    }
    fn side_remainders_cached(
        &self,
        high: bool,
        across: [f64; 2],
        budget: usize,
        cache: &mut SideRemainderCache,
    ) -> Result<(Option<[[[f64; 2]; 3]; 4]>, usize)> {
        let a = self.bridge.section(false);
        let b = self.bridge.section(true);
        let mut domain = self.source_rail_domain(high, across)?;
        domain[a.axis()] = [
            a.parameter().min(b.parameter()),
            a.parameter().max(b.parameter()),
        ];
        let source = a.profile().source();
        let (knots, degree) = if a.axis() == 0 {
            (&source.knots_u, source.degree_u)
        } else {
            (&source.knots_v, source.degree_v)
        };
        if knots.iter().any(|&k| {
            k > domain[a.axis()][0]
                && k < domain[a.axis()][1]
                && knots.iter().filter(|&&v| v == k).count() + 3 > degree
        }) {
            return Ok((None, 0));
        }
        let t = I::new(across[0], across[1])?;
        let q = I::point(1.).sub(t)?;
        let h01 = t.mul(t)?.mul(I::point(3.).sub(I::point(2.).mul(t)?)?)?;
        let h11 = t.mul(t)?.mul(q)?;
        let value = t
            .mul(t)?
            .mul(t)?
            .mul(t)?
            .div(I::point(24.))?
            .add(h01.div(I::point(24.))?)?
            .add(h11.div(I::point(6.))?)?;
        let d11 = t.mul(I::point(3.).mul(t)?.sub(I::point(2.))?)?;
        let derivative = t
            .mul(t)?
            .mul(t)?
            .div(I::point(6.))?
            .add(I::point(6.).mul(t)?.mul(q)?.div(I::point(24.))?)?
            .add(I::point(d11.lo.abs().max(d11.hi.abs())).div(I::point(6.))?)?;
        let width = I::point(b.parameter()).sub(I::point(a.parameter()))?;
        let scale = width.mul(width)?.mul(width)?.mul(width)?;
        let mut out = [[[0.; 2]; 3]; 4];
        let mut spent = 0;
        for free_jet in [false, true] {
            if spent == budget {
                return Ok((None, spent));
            }
            let mut order = [0; 2];
            order[a.axis()] = 4;
            order[1 - a.axis()] = usize::from(free_jet);
            let key = (a.profile() as *const _ as usize,
                domain.map(|d| d.map(f64::to_bits)), order);
            let (fourth, used) = if let Some(value) = cache.entries.get(&key) {
                (Some(*value), 0)
            } else {
                let result = a.profile().higher_derivative_bounds(domain, order, budget - spent)?;
                if let Some(value) = result.0 {
                    if cache.entries.len() < 100000 { cache.entries.insert(key, value); }
                }
                result
            };
            spent += used;
            let Some(fourth) = fourth else {
                return Ok((None, spent));
            };
            for k in 0..3 {
                let magnitude = I::point(fourth[k][0].abs().max(fourth[k][1].abs())).mul(scale)?;
                for (deriv, factor) in [value, derivative].into_iter().enumerate() {
                    let bound = magnitude.mul(factor)?.hi.max(0.);
                    out[2 * usize::from(free_jet) + deriv][k] = [-bound, bound];
                }
            }
        }
        Ok((Some(out), spent))
    }
    /// First jets of the full side Hermite expression, not of the linear
    /// position-only correction. Shares every source query's work allowance.
    pub fn derivatives_with_transverse_correction(
        &self,
        free: [f64; 2],
        across: [f64; 2],
        budget: usize,
    ) -> Result<(Option<[[[f64; 2]; 3]; 2]>, usize)> {
        self.derivatives_with_transverse_correction_cached(
            free, across, budget, &mut SideRemainderCache::default())
    }
    pub(crate) fn derivatives_with_transverse_correction_cached(
        &self, free: [f64; 2], across: [f64; 2], budget: usize,
        cache: &mut SideRemainderCache,
    ) -> Result<(Option<[[[f64; 2]; 3]; 2]>, usize)> {
        self.bridge.validate(free, across, budget)?;
        if across == [0., 0.] || across == [1., 1.] {
            return self.bridge.derivative_bounds(free, across, budget);
        }
        let section = self.bridge.section(false);
        let range = section.range();
        for high in [false, true] {
            if free == [range[usize::from(high)]; 2] {
                let (jet, used) = section
                    .profile()
                    .derivative_bounds(self.source_rail_domain(high, across)?, budget)?;
                let Some(jet) = jet else {
                    return Ok((None, used));
                };
                let mut out = [[[0.; 2]; 3]; 2];
                for k in 0..3 {
                    out[0][k] = jet[k][1 - section.axis()];
                    let v = I::new(jet[k][section.axis()][0], jet[k][section.axis()][1])?
                        .mul(I::point(self.bridge.scales[0]))?;
                    out[1][k] = [v.lo, v.hi];
                }
                return Ok((Some(out), used));
            }
        }
        let (base, mut spent) = self
            .bridge
            .derivative_bounds_cached(free, across, budget, true, Some(cache))?;
        let Some(base) = base else {
            return Ok((None, spent));
        };
        let mut out = [[I::point(0.); 3]; 2];
        for axis in 0..2 {
            for k in 0..3 {
                out[axis][k] = I::new(base[axis][k][0], base[axis][k][1])?;
            }
        }
        let width = I::point(range[1]).sub(I::point(range[0]))?;
        let t = I::new(free[0], free[1])?
            .sub(I::point(range[0]))?
            .div(width)?;
        let q = I::point(1.).sub(t)?;
        let h = [
            q.mul(q)?.mul(I::point(1.).add(I::point(2.).mul(t)?)?)?,
            t.mul(t)?.mul(I::point(3.).sub(I::point(2.).mul(t)?)?)?,
        ];
        let g = [
            t.mul(q)?.mul(q)?.mul(width)?,
            I::point(-1.).mul(t)?.mul(t)?.mul(q)?.mul(width)?,
        ];
        let dh = [
            I::point(-6.).mul(t)?.mul(q)?.div(width)?,
            I::point(6.).mul(t)?.mul(q)?.div(width)?,
        ];
        let dg = [
            q.mul(I::point(1.).sub(I::point(3.).mul(t)?)?)?,
            t.mul(I::point(3.).mul(t)?.sub(I::point(2.))?)?,
        ];
        for high in [false, true] {
            let domain = self.source_rail_domain(high, across)?;
            let rail_free = [range[usize::from(high)]; 2];
            let mut values = Vec::new();
            let mut source_jet: Option<[[[f64; 2]; 2]; 3]> = None;
            let mut bridge_jet: Option<[[[f64; 2]; 3]; 2]> = None;
            for query in 0..8 {
                if spent == budget && query != 4 && query != 5 {
                    return Ok((None, spent));
                }
                let rail_key=(self.bridge as *const _ as usize,high,across.map(f64::to_bits),query);
                let cached=if query<2 {cache.rail_positions.get(&rail_key)} else {None};
                let (value, used) = if let Some(value)=cached {(Some(*value),0)} else {match query {
                    0 => self.source_rail(high, across, budget - spent)?,
                    1 => self.bridge.rail(high).enclose(across, budget - spent)?,
                    2 | 4 => {
                        let (j, used) = if query == 4 {
                            (source_jet, 0)
                        } else {
                            let (j, used) = section
                                .profile()
                                .derivative_bounds(domain, budget - spent)?;
                            source_jet = j;
                            (j, used)
                        };
                        let mut v = [[0.; 2]; 3];
                        if let Some(j) = j {
                            for k in 0..3 {
                                let axis = if query == 2 {
                                    1 - section.axis()
                                } else {
                                    section.axis()
                                };
                                let scale = if query == 2 {
                                    1.
                                } else {
                                    self.bridge.scales[0]
                                };
                                let x =
                                    I::new(j[k][axis][0], j[k][axis][1])?.mul(I::point(scale))?;
                                v[k] = [x.lo, x.hi];
                            }
                            (Some(v), used)
                        } else {
                            (None, used)
                        }
                    }
                    3 | 5 => {
                        let (j, used) = if query == 5 {
                            (bridge_jet, 0)
                        } else {
                            let (j, used) = self.bridge.derivative_bounds_impl(
                                rail_free,
                                across,
                                budget - spent,
                                true,
                            )?;
                            bridge_jet = j;
                            (j, used)
                        };
                        (j.map(|j| j[usize::from(query == 5)]), used)
                    }
                    6 => {
                        let (j, used) = section
                            .profile()
                            .mixed_derivative_bounds(domain, budget - spent)?;
                        let mut v = [[0.; 2]; 3];
                        if let Some(j) = j {
                            for k in 0..3 {
                                let x = I::new(j[k][0], j[k][1])?
                                    .mul(I::point(self.bridge.scales[0]))?;
                                v[k] = [x.lo, x.hi];
                            }
                            (Some(v), used)
                        } else {
                            (None, used)
                        }
                    }
                    _ => self
                        .bridge
                        .mixed_derivative_bounds(rail_free, across, budget - spent)?,
                }};
                // Only complete position hulls are shared across free domains.
                // The exact owner, rail, across interval and query stay bound.
                if query<2 {if let Some(value)=value {
                    if cache.rail_positions.len()<100000 {cache.rail_positions.insert(rail_key,value);}
                }}
                spent += used;
                let Some(value) = value else {
                    return Ok((None, spent));
                };
                values.push(value);
            }
            let (remainders, used) = self.side_remainders_cached(high, across, budget - spent, cache)?;
            spent += used;
            let i = usize::from(high);
            for k in 0..3 {
                let v = |i: usize| I::new(values[i][k][0], values[i][k][1]);
                let mut errors = [
                    v(0)?.sub(v(1)?)?,
                    v(4)?.sub(v(5)?)?,
                    v(2)?.sub(v(3)?)?,
                    v(6)?.sub(v(7)?)?,
                ];
                if let Some(remainders) = remainders {
                    for j in 0..4 {
                        errors[j] =
                            errors[j].intersect(remainders[j][k][0], remainders[j][k][1])?;
                    }
                } else if spent == budget {
                    return Ok((None, spent));
                }
                out[0][k] = out[0][k]
                    .add(dh[i].mul(errors[0])?)?
                    .add(dg[i].mul(errors[2])?)?;
                out[1][k] = out[1][k]
                    .add(h[i].mul(errors[1])?)?
                    .add(g[i].mul(errors[3])?)?;
            }
        }
        Ok((Some(out.map(|axis| axis.map(|v| [v.lo, v.hi]))), spent))
    }
    pub fn normals_with_transverse_correction(
        &self,
        free: [f64; 2],
        across: [f64; 2],
        budget: usize,
    ) -> Result<(Option<[[f64; 2]; 3]>, usize)> {
        let (jet, spent) = self.derivatives_with_transverse_correction(free, across, budget)?;
        let Some([a, b]) = jet else {
            return Ok((None, spent));
        };
        let mut bounds = [[0.; 2]; 3];
        for k in 0..3 {
            let i = (k + 1) % 3;
            let j = (k + 2) % 3;
            let n = I::new(a[i][0], a[i][1])?
                .mul(I::new(b[j][0], b[j][1])?)?
                .sub(I::new(a[j][0], a[j][1])?.mul(I::new(b[i][0], b[i][1])?)?)?;
            bounds[k] = [n.lo, n.hi];
        }
        Ok((Some(bounds), spent))
    }
    pub fn inspect_transverse_corrected_regularity(
        &self,
        free: [f64; 2],
        across: [f64; 2],
        max_cells: usize,
        max_spans: usize,
    ) -> Result<crate::surface_weight_profile::TransitionRegularityReport> {
        self.bridge.validate(free, across, max_spans)?;
        check(max_cells <= 100000, "Bound bridge cell work")?;
        let mut out = crate::surface_weight_profile::TransitionRegularityReport {
            regular: Vec::new(),
            pending: Vec::new(),
            cells: 0,
            spans: 0,
            complete_regular: false,
            reason: "transverse-corrected-bridge-regularity-work-limit",
        };
        let mut queue = std::collections::VecDeque::from([[free, across]]);
        while let Some(rect) = queue.pop_front() {
            if out.cells == max_cells || out.spans == max_spans {
                out.pending.push(rect);
                out.pending.extend(queue);
                return Ok(out);
            }
            out.cells += 1;
            let (bounds, spent) =
                self.normals_with_transverse_correction(rect[0], rect[1], max_spans - out.spans)?;
            out.spans += spent;
            let Some(bounds) = bounds else {
                out.pending.push(rect);
                out.pending.extend(queue);
                return Ok(out);
            };
            if bounds.iter().any(|r| r[0] > 0. || r[1] < 0.) {
                out.regular.push(rect);
                continue;
            }
            let range = self.bridge.section(false).range();
            let axis = usize::from(
                (rect[0][1] - rect[0][0]) / (range[1] - range[0]) < rect[1][1] - rect[1][0],
            );
            let mid = rect[axis][0] * 0.5 + rect[axis][1] * 0.5;
            if mid <= rect[axis][0] || mid >= rect[axis][1] {
                out.pending.push(rect);
                continue;
            }
            let mut left = rect;
            left[axis][1] = mid;
            let mut right = rect;
            right[axis][0] = mid;
            // Finish refining a local rectangle before distributing work over
            // every unresolved breadth level. The untouched sibling remains
            // in the queue, preserving complete requested-domain coverage.
            queue.push_front(right);
            queue.push_front(left);
        }
        out.complete_regular = out.pending.is_empty();
        out.reason = if out.complete_regular {
            "transverse-corrected-bridge-regularity-qualified"
        } else {
            "transverse-corrected-bridge-regularity-resolution-limit"
        };
        Ok(out)
    }
    /// Prove injectivity only with one fixed projection and complete coverage
    /// of the corrected expression's projected Jacobian principal minors.
    pub fn inspect_transverse_corrected_injectivity(
        &self, free:[f64;2], across:[f64;2], projection:[[f64;3];2],
        max_cells:usize, max_spans:usize,
    ) -> Result<crate::surface_weight_profile::TransitionRegularityReport> {
        self.inspect_transverse_corrected_injectivity_policy(
            free,across,projection,max_cells,max_spans,false)
    }
    /// Spend additional shared work on first jets integrated from source rails.
    pub fn inspect_transverse_corrected_injectivity_from_rails(
        &self, free:[f64;2], across:[f64;2], projection:[[f64;3];2],
        max_cells:usize, max_spans:usize,
    ) -> Result<crate::surface_weight_profile::TransitionRegularityReport> {
        self.inspect_transverse_corrected_injectivity_policy(
            free,across,projection,max_cells,max_spans,true)
    }
    fn inspect_transverse_corrected_injectivity_policy(
        &self, free:[f64;2], across:[f64;2], projection:[[f64;3];2],
        max_cells:usize, max_spans:usize, tighten:bool,
    ) -> Result<crate::surface_weight_profile::TransitionRegularityReport> {
        self.bridge.validate(free, across, max_spans)?;
        check(
            max_cells <= 100000 && projection.iter().flatten().all(|x| x.is_finite()),
            "Bound injectivity work and projection",
        )?;
        let mut out = crate::surface_weight_profile::TransitionRegularityReport {
            regular: Vec::new(),
            pending: Vec::new(),
            cells: 0,
            spans: 0,
            complete_regular: false,
            reason: "transverse-corrected-bridge-injectivity-work-limit",
        };
        // The transverse field must be single-valued and piecewise C1. Refuse
        // internal knots whose multiplicity would allow first-derivative jumps.
        for high in [false, true] {
            let surface = self.bridge.section(high).profile().source();
            for (knots, degree, count) in [
                (
                    &surface.knots_u,
                    surface.degree_u,
                    surface.control_points.len(),
                ),
                (
                    &surface.knots_v,
                    surface.degree_v,
                    surface.control_points[0].len(),
                ),
            ] {
                if knots.iter().any(|&k| {
                    k > knots[degree]
                        && k < knots[count]
                        && knots.iter().filter(|&&x| x == k).count() >= degree
                }) {
                    out.pending.push([free, across]);
                    out.reason = "transverse-corrected-bridge-injectivity-continuity-unproven";
                    return Ok(out);
                }
            }
        }
        let mut queue = std::collections::VecDeque::from([[free, across]]);
        while let Some(rect) = queue.pop_front() {
            if out.cells == max_cells || out.spans == max_spans {
                out.pending.push(rect);
                out.pending.extend(queue);
                return Ok(out);
            }
            out.cells += 1;
            let (jet, spent) = self.derivatives_with_transverse_correction(
                rect[0],
                rect[1],
                max_spans - out.spans,
            )?;
            out.spans += spent;
            let Some(mut jet) = jet else {
                out.pending.push(rect);
                out.pending.extend(queue);
                return Ok(out);
            };
            let mut j = [[I::point(0.); 2]; 2];
            for row in 0..2 {
                for axis in 0..2 {
                    for k in 0..3 {
                        j[row][axis] = j[row][axis].add(
                            I::point(projection[row][k])
                                .mul(I::new(jet[axis][k][0], jet[axis][k][1])?)?,
                        )?;
                    }
                }
            }
            if projected_principal_minors_positive(j)? {
                out.regular.push(rect);
                continue;
            }
            if tighten && out.spans < max_spans {
                let (parts, spent) = self.free_jet_from_rail_parts(
                    rect[0], rect[1], max_spans-out.spans)?;
                out.spans += spent;
                if let Some((anchor,second,delta)) = parts {
                    let mut anchored=[[0.;2];3];
                    for k in 0..3 {
                        let v=I::new(anchor[k][0],anchor[k][1])?.add(
                            I::new(second[k][0],second[k][1])?.mul(delta)?)?;
                        anchored[k]=[v.lo,v.hi];
                    }
                    for k in 0..3 {
                        let common = I::new(jet[0][k][0],jet[0][k][1])?
                            .intersect(anchored[k][0],anchored[k][1])?;
                        jet[0][k]=[common.lo,common.hi];
                    }
                    for row in 0..2 {
                        j[row][0]=I::point(0.);
                        for k in 0..3 {
                            j[row][0]=j[row][0].add(I::point(projection[row][k])
                                .mul(I::new(jet[0][k][0],jet[0][k][1])?)?)?;
                        }
                    }
                    let projected=second_free_jet::projected_rail_column(projection,anchor,second,delta)?;
                    for row in 0..2 {
                        j[row][0]=j[row][0].intersect(projected[row].lo,projected[row].hi)?;
                    }
                    if projected_principal_minors_positive(j)? ||
                        (j[0][0].lo>0. && j[1][1].lo>0. &&
                            second_free_jet::projected_rail_determinant(projection,anchor,second,
                                delta,[j[0][1],j[1][1]])?.lo>0.) {
                        out.regular.push(rect);
                        continue;
                    }
                }
            }
            let range = self.bridge.section(false).range();
            let axis = usize::from(
                (rect[0][1] - rect[0][0]) / (range[1] - range[0]) < rect[1][1] - rect[1][0],
            );
            let mid = rect[axis][0] * 0.5 + rect[axis][1] * 0.5;
            if mid <= rect[axis][0] || mid >= rect[axis][1] {
                out.pending.push(rect);
                continue;
            }
            let mut left = rect;
            left[axis][1] = mid;
            let mut right = rect;
            right[axis][0] = mid;
            if tighten {
                // Cover the whole domain before refining one difficult rail
                // corner to machine precision. Pending cells stay explicit.
                queue.push_back(left);
                queue.push_back(right);
            } else {
                queue.push_front(right);
                queue.push_front(left);
            }
        }
        out.complete_regular = out.pending.is_empty();
        out.reason = if out.complete_regular {
            "transverse-corrected-bridge-injectivity-qualified"
        } else {
            "transverse-corrected-bridge-injectivity-resolution-limit"
        };
        Ok(out)
    }
    /// Exact side first-jet identity is established by the full Hermite
    /// expression. Only regular source support normals remain to be enclosed.
    /// This checks the original profile support, not neighboring Body ownership.
    pub fn inspect_transverse_corrected_section_join(&self,high:bool,free:[f64;2],
        max_cells:usize,max_spans:usize
    )->Result<crate::surface_weight_profile::TransitionRegularityReport> {
        let across=[if high {1.}else{0.};2];
        self.bridge.validate(free,across,max_spans)?;
        let section=self.bridge.section(high);
        // The full correction vanishes with its first jet at across=0/1;
        // exact section position and the nonzero scaled transverse derivative
        // are inherited. Coverage below proves source support regularity.
        let mut domain=[[section.parameter();2];2];
        domain[1-section.axis()]=free;
        let mut report=section.profile().inspect_regularity(domain,max_cells,max_spans)?;
        report.reason=if report.complete_regular {"full-section-join-qualified"}else{"full-section-join-regularity-unproven"};
        Ok(report)
    }

    /// Cover a free-end rail of the exact full corrected expression. Body
    /// ownership and neighbor angle remain separate qualification steps.
    pub fn inspect_transverse_corrected_side_join(
        &self,
        high: bool,
        across: [f64; 2],
        max_cells: usize,
        max_spans: usize,
    ) -> Result<crate::surface_weight_profile::TransitionRegularityReport> {
        let free = self.bridge.section(false).range()[usize::from(high)];
        self.bridge.validate([free; 2], across, max_spans)?;
        check(max_cells <= 100000, "Bound full side join work")?;
        let mut out = crate::surface_weight_profile::TransitionRegularityReport {
            regular: Vec::new(),
            pending: Vec::new(),
            cells: 0,
            spans: 0,
            complete_regular: false,
            reason: "full-side-join-work-limit",
        };
        let mut queue = std::collections::VecDeque::from([[[free; 2], across]]);
        while let Some(rect) = queue.pop_front() {
            if out.cells == max_cells || out.spans == max_spans {
                out.pending.push(rect);
                out.pending.extend(queue);
                return Ok(out);
            }
            out.cells += 1;
            let (normal, used) = self.bridge.section(false).profile().normal_bounds(
                self.source_rail_domain(high, rect[1])?,
                max_spans - out.spans,
            )?;
            out.spans += used;
            let Some(normal) = normal else {
                out.pending.push(rect);
                out.pending.extend(queue);
                return Ok(out);
            };
            if normal.iter().any(|r| r[0] > 0. || r[1] < 0.) {
                out.regular.push(rect);
                continue;
            }
            let mid = rect[1][0] * 0.5 + rect[1][1] * 0.5;
            if mid <= rect[1][0] || mid >= rect[1][1] {
                out.pending.push(rect);
                continue;
            }
            let mut left = rect;
            left[1][1] = mid;
            let mut right = rect;
            right[1][0] = mid;
            queue.push_back(left);
            queue.push_back(right);
        }
        out.complete_regular = out.pending.is_empty();
        out.reason = if out.complete_regular {
            "full-side-join-qualified"
        } else {
            "full-side-join-resolution-limit"
        };
        Ok(out)
    }
    pub fn derivative_bounds(
        &self,
        free: [f64; 2],
        across: [f64; 2],
        budget: usize,
    ) -> Result<(Option<[[[f64; 2]; 3]; 2]>, usize)> {
        self.bridge.validate(free, across, budget)?;
        if across == [0., 0.] || across == [1., 1.] {
            return self.bridge.derivative_bounds(free, across, budget);
        }
        let (base, mut spent) = self.bridge.derivative_bounds(free, across, budget)?;
        let Some(base) = base else {
            return Ok((None, spent));
        };
        let mut result = [[I::point(0.); 3]; 2];
        for axis in 0..2 {
            for k in 0..3 {
                result[axis][k] = I::new(base[axis][k][0], base[axis][k][1])?;
            }
        }
        let range = self.bridge.section(false).range();
        let width = I::point(range[1]).sub(I::point(range[0]))?;
        let t = I::new(free[0], free[1])?
            .sub(I::point(range[0]))?
            .div(width)?;
        for high in [false, true] {
            let mut values = Vec::new();
            for query in 0..4 {
                if spent == budget {
                    return Ok((None, spent));
                }
                let (bounds, used) = match query {
                    0 => self.source_rail(high, across, budget - spent)?,
                    1 => self.bridge.rail(high).enclose(across, budget - spent)?,
                    2 => {
                        let section = self.bridge.section(false);
                        let (j, used) = section.profile().derivative_bounds(
                            self.source_rail_domain(high, across)?,
                            budget - spent,
                        )?;
                        let mut bounds = [[0.; 2]; 3];
                        if let Some(j) = j {
                            for k in 0..3 {
                                let value =
                                    I::new(j[k][section.axis()][0], j[k][section.axis()][1])?
                                        .mul(I::point(self.bridge.scales[0]))?;
                                bounds[k] = [value.lo, value.hi];
                            }
                            (Some(bounds), used)
                        } else {
                            (None, used)
                        }
                    }
                    _ => {
                        let (j, used) = self.bridge.derivative_bounds(
                            [range[usize::from(high)]; 2],
                            across,
                            budget - spent,
                        )?;
                        (j.map(|j| j[1]), used)
                    }
                };
                spent += used;
                let Some(bounds) = bounds else {
                    return Ok((None, spent));
                };
                values.push(bounds);
            }
            let weight = if high { t } else { I::point(1.).sub(t)? };
            let sign = I::point(if high { 1. } else { -1. }).div(width)?;
            for k in 0..3 {
                let v = |q: usize| I::new(values[q][k][0], values[q][k][1]);
                result[0][k] = result[0][k].add(sign.mul(v(0)?.sub(v(1)?)?)?)?;
                result[1][k] = result[1][k].add(weight.mul(v(2)?.sub(v(3)?)?)?)?;
            }
        }
        Ok((Some(result.map(|axis| axis.map(|r| [r.lo, r.hi]))), spent))
    }
    pub fn normal_bounds(
        &self,
        free: [f64; 2],
        across: [f64; 2],
        budget: usize,
    ) -> Result<(Option<[[f64; 2]; 3]>, usize)> {
        let (j, spent) = self.derivative_bounds(free, across, budget)?;
        let Some([a, b]) = j else {
            return Ok((None, spent));
        };
        let mut bounds = [[0.; 2]; 3];
        for k in 0..3 {
            let i = (k + 1) % 3;
            let l = (k + 2) % 3;
            let n = I::new(a[i][0], a[i][1])?
                .mul(I::new(b[l][0], b[l][1])?)?
                .sub(I::new(a[l][0], a[l][1])?.mul(I::new(b[i][0], b[i][1])?)?)?;
            bounds[k] = [n.lo, n.hi];
        }
        Ok((Some(bounds), spent))
    }
    pub fn inspect_support_rail_normals(
        &self,
        high: bool,
        across: [f64; 2],
        threshold: f64,
        budget: usize,
    ) -> Result<crate::normal_alignment::PairReport> {
        check(
            threshold.is_finite() && (0. ..1.).contains(&threshold),
            "Bound constrained rail angle",
        )?;
        let free = self.bridge.section(false).range()[usize::from(high)];
        let (a, mut spent) = self.normal_bounds([free; 2], across, budget)?;
        let mut out = crate::normal_alignment::PairReport {
            aligned: None,
            sine_squared_interval: None,
            normal_components: [a, None],
            spans: spent,
            reason: "constrained-rail-normal-work-limit",
        };
        let Some(a) = a else {
            return Ok(out);
        };
        if spent == budget {
            return Ok(out);
        }
        let (b, used) = self
            .bridge
            .section(false)
            .profile()
            .normal_bounds(self.source_rail_domain(high, across)?, budget - spent)?;
        spent += used;
        let Some(b) = b else {
            out.spans = spent;
            return Ok(out);
        };
        let mut compared = crate::normal_alignment::inspect_pair_bounds([a, b], threshold)?;
        compared.spans = spent;
        Ok(compared)
    }
    pub fn enclose(
        &self,
        free: [f64; 2],
        across: [f64; 2],
        budget: usize,
    ) -> Result<(Option<[[f64; 2]; 3]>, usize)> {
        self.bridge.validate(free, across, budget)?;
        if across == [0., 0.] || across == [1., 1.] {
            return self.bridge.enclose(free, across, budget);
        }
        let range = self.bridge.section(false).range();
        for high in [false, true] {
            if free == [range[usize::from(high)]; 2] {
                return self.source_rail(high, across, budget);
            }
        }
        let (base, mut spent) = self.bridge.enclose(free, across, budget)?;
        let Some(base) = base else {
            return Ok((None, spent));
        };
        let mut result = base
            .map(|r| I::new(r[0], r[1]))
            .into_iter()
            .collect::<Result<Vec<_>>>()?;
        let t = I::new(free[0], free[1])?
            .sub(I::point(range[0]))?
            .div(I::point(range[1]).sub(I::point(range[0]))?)?;
        for high in [false, true] {
            if spent == budget {
                return Ok((None, spent));
            }
            let (rail, used) = self.source_rail(high, across, budget - spent)?;
            spent += used;
            let Some(rail) = rail else {
                return Ok((None, spent));
            };
            if spent == budget {
                return Ok((None, spent));
            }
            let (old, used) = self.bridge.rail(high).enclose(across, budget - spent)?;
            spent += used;
            let Some(old) = old else {
                return Ok((None, spent));
            };
            let weight = if high { t } else { I::point(1.).sub(t)? };
            for k in 0..3 {
                let error = I::new(rail[k][0], rail[k][1])?.sub(I::new(old[k][0], old[k][1])?)?;
                result[k] = result[k].add(weight.mul(error)?)?;
            }
        }
        Ok((
            Some(std::array::from_fn(|k| [result[k].lo, result[k].hi])),
            spent,
        ))
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn side_remainder_cache_reuses_complete_source_hulls_without_changing_jets() {
        let surface = crate::surface::Surface {
            degree_u: 2, degree_v: 1,
            knots_u: vec![0.,0.,0.,1.,1.,1.], knots_v: vec![0.,0.,1.,1.],
            control_points: [0.,0.5,1.].into_iter().map(|u|
                vec![vec![u,0.,u*u],vec![u,1.,u*u]]).collect(),
            weights: [1.,0.75,1.25].into_iter().map(|w|vec![w;2]).collect(),
            periodic_u: false, periodic_v: false,
        };
        let profile = crate::normal_alignment::qualify_weight_profile(&surface,[0,0],100).unwrap().0.unwrap();
        let bridge = ProfileSectionBridge::new(profile.section(0,0.).unwrap(),
            profile.section(0,1.).unwrap(),[1.;2]).unwrap();
        let constrained = bridge.constrain_original_rails(100).unwrap().0.unwrap();
        let mut cache = SideRemainderCache::default();
        assert_eq!(constrained.derivatives_with_transverse_correction_cached(
            [0.2,0.8],[0.1,0.4],0,&mut cache).unwrap(),(None,0));
        assert!(cache.entries.is_empty());
        assert!(cache.bridge_data.is_empty());
        assert!(cache.rail_positions.is_empty());
        let cold = constrained.derivatives_with_transverse_correction_cached(
            [0.2,0.8],[0.1,0.4],1000,&mut cache).unwrap();
        assert!(cold.0.is_some()); assert_eq!(cache.entries.len(),4);
        assert_eq!(cache.bridge_data.len(),1);
        assert_eq!(cache.rail_positions.len(),4);
        let direct_position=constrained.enclose_with_transverse_correction([0.3,0.7],[0.1,0.4],1000).unwrap();
        let shared_position=constrained.enclose_with_transverse_correction_cached([0.3,0.7],[0.1,0.4],1000,Some(&cache)).unwrap();
        assert_eq!(shared_position.0,direct_position.0);
        assert!(shared_position.1<direct_position.1,"Complete rail positions must survive a free-only domain change");
        for across in [[0.5,0.9],[0.25,0.75]] {
            let direct=bridge.enclose([0.2,0.8],across,1000).unwrap();
            let reused=bridge.enclose_cached([0.2,0.8],across,1000,Some(&cache)).unwrap();
            assert_eq!(reused.0,direct.0);
            assert!(reused.1<direct.1);
            assert_eq!(bridge.enclose_cached([0.2,0.8],across,0,Some(&cache)).unwrap(),(None,0));
            let direct=constrained.enclose_with_transverse_correction([0.2,0.8],across,1000).unwrap();
            let reused=constrained.enclose_with_transverse_correction_cached([0.2,0.8],across,1000,Some(&cache)).unwrap();
            assert_eq!(reused.0,direct.0);
            assert!(reused.1<direct.1);
        }
        for across in [[0.5,0.9],[0.25,0.75]] {
            let direct=bridge.derivative_bounds_impl([0.2,0.8],across,1000,true).unwrap();
            let reused=bridge.derivative_bounds_cached([0.2,0.8],across,1000,true,Some(&mut cache)).unwrap();
            assert_eq!(reused.0,direct.0);
            assert!(reused.1<direct.1,"Section data must survive an across-only change");
            assert_eq!(cache.bridge_data.len(),1);
        }
        let other=ProfileSectionBridge::new(profile.section(0,0.25).unwrap(),
            profile.section(0,0.75).unwrap(),[2.;2]).unwrap();
        assert_eq!(other.enclose_cached([0.2,0.8],[0.2,0.7],1000,Some(&cache)).unwrap(),
            other.enclose([0.2,0.8],[0.2,0.7],1000).unwrap(),
            "Position reuse must not cross section owners");
        let direct=other.derivative_bounds_impl([0.2,0.8],[0.2,0.7],1000,true).unwrap();
        let isolated=other.derivative_bounds_cached([0.2,0.8],[0.2,0.7],1000,true,Some(&mut cache)).unwrap();
        assert_eq!(isolated,direct,"Different section owners must not share cached data");
        assert_eq!(cache.bridge_data.len(),2);
        for across in [[0.5,0.9],[0.25,0.75],[0.4,0.4]] {
            let direct = constrained.derivatives_with_transverse_correction([0.3,0.7],across,1000).unwrap();
            let warm = constrained.derivatives_with_transverse_correction_cached(
                [0.3,0.7],across,1000,&mut cache).unwrap();
            assert_eq!(warm.0,direct.0); assert!(warm.0.is_some());
            assert!(warm.1 < direct.1,"warm={} direct={}",warm.1,direct.1);
        }
        assert!(constrained.derivatives_with_transverse_correction_cached(
            [0.2,0.8],[-0.1,0.4],1000,&mut cache).is_err());
        assert_eq!(constrained.derivatives_with_transverse_correction_cached(
            [0.2,0.8],[0.1,0.4],0,&mut cache).unwrap(),(None,0));
        // A bounded failed attempt may retain complete subqueries, but never
        // a partial fourth-derivative enclosure or a completed corrected jet.
        let mut limited = SideRemainderCache::default();
        assert!(constrained.side_remainders_cached(false,[0.1,0.4],1,&mut limited).unwrap().0.is_none());
        assert_eq!(limited.entries.len(),1);
        let retry = constrained.side_remainders_cached(false,[0.5,0.9],1000,&mut limited).unwrap();
        assert_eq!(retry.0,constrained.side_remainders(false,[0.5,0.9],1000).unwrap().0);
        assert_eq!(limited.entries.len(),2);
        let twin = crate::normal_alignment::qualify_weight_profile(&surface,[0,0],100).unwrap().0.unwrap();
        let other = ProfileSectionBridge::new(twin.section(0,0.).unwrap(),twin.section(0,1.).unwrap(),[1.;2]).unwrap();
        let other = other.constrain_original_rails(100).unwrap().0.unwrap();
        let foreign = other.side_remainders_cached(false,[0.5,0.9],1000,&mut cache).unwrap();
        assert!(foreign.1>0,"Distinct owners must compute their own source hulls");
    }
    #[test]
    fn transverse_bezier_hull_covers_independent_curved_hermite_derivatives() {
        let controls=[[0.,0.,0.],[1.,2.,-1.],[2.,-2.,3.],[3.,1.,1.]];
        let scales=[0.75,1.25];
        for shift in [0.,1e6] {
            let surface=crate::surface::Surface {
                degree_u:3,degree_v:1,knots_u:vec![0.,0.,0.,0.,1.,1.,1.,1.],
                knots_v:vec![0.,0.,1.,1.],
                control_points:controls.iter().map(|p|[0.,1.].into_iter().map(|v|
                    vec![p[0]+shift,p[1]+v-shift,p[2]+2.*v+shift]).collect()).collect(),
                weights:vec![vec![1.;2];4],periodic_u:false,periodic_v:false,
            };
            let profile=crate::normal_alignment::qualify_weight_profile(&surface,[0,0],8).unwrap().0.unwrap();
            let bridge=ProfileSectionBridge::new(profile.section(0,0.).unwrap(),
                profile.section(0,1.).unwrap(),scales).unwrap();
            for range in [[0.,1.],[0.125,0.75],[0.5,0.5]] {
                for common in [false,true] {
                    let (jet,spent)=bridge.derivative_bounds_impl([0.2,0.8],range,100,common).unwrap();
                    let jet=jet.unwrap();assert!(spent<=100);
                    for k in 0..3 {
                        let expected=[0.,1.,2.][k];
                        assert!(jet[0][k][0]<=expected && expected<=jet[0][k][1]);
                        assert!(jet[0][k][1]-jet[0][k][0]<1e-8,
                            "cubic free derivative must retain constant extrusion tangent");
                    }
                    for i in 0..=64 {
                        let t=range[0]+(range[1]-range[0])*i as f64/64.;let q=1.-t;
                        for k in 0..3 {
                            let d0=scales[0]*3.*(controls[1][k]-controls[0][k]);
                            let d1=scales[1]*3.*(controls[3][k]-controls[2][k]);
                            let expected=6.*t*q*(controls[3][k]-controls[0][k])
                                +q*(1.-3.*t)*d0+t*(3.*t-2.)*d1;
                            assert!(jet[1][k][0]<=expected && expected<=jet[1][k][1],
                                "shift={shift} common={common} range={range:?} k={k} t={t} misses {expected}");
                        }
                    }
                }
            }
            assert!(bridge.derivative_bounds_impl([0.2,0.8],[0.,1.],1,true).unwrap().0.is_none());
        }
    }
    #[test]
    fn independent_profile_chords_cover_plane_and_preserve_folded_candidate() {
        let a=crate::surface::Surface {
            degree_u:1,degree_v:1,knots_u:vec![0.,0.,1.,1.],knots_v:vec![0.,0.,1.,1.],
            control_points:[0.,1.].into_iter().map(|u|vec![vec![u,0.,0.],vec![u,1.,0.]]).collect(),
            weights:vec![vec![1.;2];2],periodic_u:false,periodic_v:false,
        };
        let b=a.clone();
        let pa=crate::normal_alignment::qualify_weight_profile(&a,[0,0],100).unwrap().0.unwrap();
        let pb=crate::normal_alignment::qualify_weight_profile(&b,[0,0],100).unwrap().0.unwrap();
        let bridge=ProfileSectionBridge::new(pa.section(0,0.).unwrap(),pb.section(0,1.).unwrap(),[1.;2]).unwrap();
        assert!(bridge.constrain_original_rails(100).unwrap().0.is_none());
        assert_eq!(bridge.inspect_self_chords([[0.,1.];2],1e-4,0).unwrap(),(None,0));
        for i in 0..16 {for j in 0..16 {
            let domain=[[j as f64/16.,(j+1) as f64/16.],[i as f64/16.,(i+1) as f64/16.]];
            let (excluded,spent)=bridge.inspect_self_chords(domain,1e-4,100).unwrap();
            assert_eq!(excluded,Some(true),"domain={domain:?}");assert!(spent<=100);
        }}
        let zero=bridge.inspect_self_chord_coverage([[0.,1.];2],1e-4,0,1000).unwrap();
        assert_eq!(zero.unresolved_pairs,vec![[[[0.,1.];2];2]]);
        assert!(!zero.complete_exclusion);assert_eq!(zero.cells,0);
        let limited=bridge.inspect_self_chord_coverage([[0.,1.];2],1e-4,1,1000).unwrap();
        let mass:f64=limited.unresolved_pairs.iter().map(|p| {
            let volume:f64=p.iter().flatten().map(|r|r[1]-r[0]).product();
            volume*if p[0]==p[1]{1.}else{2.}
        }).sum();
        assert_eq!(mass,1.,"Unresolved symmetric products preserve complete coverage");
        let all=bridge.inspect_self_chord_coverage([[0.,1.];2],1e-4,10000,100000).unwrap();
        assert!(all.complete_exclusion,"plane unresolved={:?}",all.unresolved_pairs);
        assert!(all.cells<=10000 && all.spans<=100000);
        let mut shifted=a.clone();
        shifted.knots_u=vec![10.,10.,12.,12.];shifted.knots_v=vec![-2.,-2.,6.,6.];
        let shifted_twin=shifted.clone();
        let sp=crate::normal_alignment::qualify_weight_profile(&shifted,[0,0],100).unwrap().0.unwrap();
        let sq=crate::normal_alignment::qualify_weight_profile(&shifted_twin,[0,0],100).unwrap().0.unwrap();
        let reparameterized=ProfileSectionBridge::new(sp.section(0,10.).unwrap(),sq.section(0,12.).unwrap(),[2.;2]).unwrap();
        assert!(reparameterized.inspect_self_chord_coverage([[-2.,6.],[0.,1.]],1e-4,10000,100000).unwrap().complete_exclusion);
        for (domain,sine,cells,spans) in [([[0.,0.];2],1e-4,0,0),([[0.,1.];2],1.,0,0),
            ([[0.,1.];2],1e-4,100001,0),([[0.,1.];2],1e-4,0,100001)] {
            assert!(bridge.inspect_self_chord_coverage(domain,sine,cells,spans).is_err());
        }
        let arch=crate::surface::Surface {
            degree_u:1,degree_v:3,knots_u:vec![0.,0.,1.,1.],
            knots_v:vec![0.,0.,0.,0.,1.,1.,1.,1.],
            control_points:[0.,1.].into_iter().map(|x|
                [[1.,0.],[1.,2.],[-1.,2.],[-1.,0.]].into_iter().map(|p|vec![x,p[0],p[1]]).collect()).collect(),
            weights:vec![vec![1.,1./3.,1./3.,1.];2],periodic_u:false,periodic_v:false,
        };
        let twin=arch.clone();
        let left=crate::normal_alignment::qualify_weight_profile(&arch,[0,0],100).unwrap().0.unwrap();
        let right=crate::normal_alignment::qualify_weight_profile(&twin,[0,0],100).unwrap().0.unwrap();
        let cylinder=ProfileSectionBridge::new(left.section(0,0.).unwrap(),right.section(0,1.).unwrap(),[1.;2]).unwrap();
        // The rational cross section has opposite endpoints and endpoint
        // tangents perpendicular to their nonzero normal diameter.
        for budget in [0,1,8,64,256] {
            let (excluded,spent)=cylinder.inspect_self_chords([[0.,1.];2],1e-4,budget).unwrap();
            assert_ne!(excluded,Some(true));assert!(spent<=budget);
        }
        let full=cylinder.inspect_self_chord_coverage([[0.,1.];2],1e-4,100,10000).unwrap();
        assert!(!full.complete_exclusion);
        let contains=|r:[[f64;2];2],p:[f64;2]|(0..2).all(|k|r[k][0]<=p[k] && p[k]<=r[k][1]);
        assert!(full.unresolved_pairs.iter().any(|p|
            (contains(p[0],[0.,0.4]) && contains(p[1],[1.,0.4])) ||
            (contains(p[1],[0.,0.4]) && contains(p[0],[1.,0.4]))),"Actual normal diameter remains covered");
        let encloses=|a:[[[f64;2];2];2],b:[[[f64;2];2];2]|(0..2).all(|e|
            (0..2).all(|k|a[e][k][0]<=b[e][k][0] && b[e][k][1]<=a[e][k][1]));
        let mut previous=vec![[[[0.,1.];2];2]];
        for budget in [0,1,7,31,100] {
            let refined=cylinder.inspect_self_chord_coverage([[0.,1.];2],1e-4,budget,10000).unwrap();
            assert!(refined.cells<=budget && refined.spans<=10000);
            for pair in &refined.unresolved_pairs {
                assert!(previous.iter().any(|old|encloses(*old,*pair)||encloses(*old,[pair[1],pair[0]])),
                    "Increased work must preserve endpoint coverage inside prior pending obligations");
            }
            previous=refined.unresolved_pairs;
        }
        let separated=[[[0.1,0.11],[0.1,0.11]],[[0.2,0.21],[0.2,0.21]]];
        for pair in [separated,[separated[1],separated[0]]] {
            assert_eq!(bridge.inspect_self_chord_pair(pair,1e-4,1000).unwrap().0,Some(true));
            assert_eq!(bridge.inspect_self_chord_pair(pair,1e-4,0).unwrap(),(None,0));
        }
        let diameter=[[[0.,0.001],[0.4,0.41]],[[0.999,1.],[0.4,0.41]]];
        for pair in [diameter,[diameter[1],diameter[0]]] {for budget in [0,1,8,64,1000] {
            let (excluded,spent)=cylinder.inspect_self_chord_pair(pair,1e-4,budget).unwrap();
            assert_ne!(excluded,Some(true));assert!(spent<=budget);
        }}
        let kink=crate::surface::Surface {
            degree_u:1,degree_v:1,knots_u:vec![0.,0.,1.,1.],knots_v:vec![0.,0.,0.5,1.,1.],
            control_points:[0.,1.].into_iter().map(|u|vec![vec![u,0.,0.],vec![u,0.5,0.],vec![u,1.,0.3]]).collect(),
            weights:vec![vec![1.;3];2],periodic_u:false,periodic_v:false,
        };
        let kp=crate::normal_alignment::qualify_weight_profile(&kink,[0,0],100).unwrap().0.unwrap();
        let discontinuous=ProfileSectionBridge::new(kp.section(0,0.).unwrap(),kp.section(0,1.).unwrap(),[1.;2]).unwrap();
        assert_eq!(discontinuous.inspect_self_chords([[0.,1.];2],1e-4,1000).unwrap(),(None,0));
        assert_eq!(discontinuous.inspect_self_chord_pair(separated,1e-4,1000).unwrap(),(None,0));
        for bad in [[[[0.,0.],[0.,1.]],separated[1]],[[[f64::NAN,0.1],[0.,1.]],separated[1]],
            [[[0.,1.1],[0.,1.]],separated[1]]] {
            assert!(bridge.inspect_self_chord_pair(bad,1e-4,0).is_err());
        }
        assert!(bridge.inspect_self_chord_pair(separated,1.,100).is_err());
        assert!(bridge.inspect_self_chord_pair(separated,1e-4,100001).is_err());
        let folded=ProfileSectionBridge::new(pa.section(0,0.).unwrap(),pb.section(0,0.).unwrap(),[1.;2]).unwrap();
        assert_ne!(folded.inspect_self_chords([[0.,1.];2],1e-4,100).unwrap().0,Some(true));
        for (domain,angle,budget) in [([[-0.1,1.],[0.,1.]],1e-4,100),
            ([[0.,1.],[0.,0.]],1e-4,100),([[0.,1.];2],1.,100),([[0.,1.];2],1e-4,100001)] {
            assert!(bridge.inspect_self_chords(domain,angle,budget).is_err());
        }
    }
    #[test]
    fn mixed_bezier_hull_covers_varying_free_tangent_field() {
        let c=[[0.,0.,0.],[1.,2.,-1.],[2.,-2.,3.],[3.,1.,1.]];
        let d=[[0.,1.,2.],[1.,-1.,3.],[2.,2.,0.],[-1.,1.,-2.]];
        let scales=[0.75,1.25];
        let surface=crate::surface::Surface {
            degree_u:3,degree_v:1,knots_u:vec![0.,0.,0.,0.,1.,1.,1.,1.],
            knots_v:vec![0.,0.,1.,1.],
            control_points:(0..4).map(|i|[0.,1.].into_iter().map(|v|
                (0..3).map(|k|c[i][k]+v*d[i][k]).collect()).collect()).collect(),
            weights:vec![vec![1.;2];4],periodic_u:false,periodic_v:false,
        };
        let profile=crate::normal_alignment::qualify_weight_profile(&surface,[0,0],8).unwrap().0.unwrap();
        let bridge=ProfileSectionBridge::new(profile.section(0,0.).unwrap(),
            profile.section(0,1.).unwrap(),scales).unwrap();
        for range in [[0.,1.],[0.125,0.75],[0.5,0.5]] {
            let (mixed,spent)=bridge.mixed_derivative_bounds([0.2,0.8],range,100).unwrap();
            let mixed=mixed.unwrap();assert!(spent<=100);
            for i in 0..=64 {
                let t=range[0]+(range[1]-range[0])*i as f64/64.;let q=1.-t;
                for k in 0..3 {
                    let m0=scales[0]*3.*(d[1][k]-d[0][k]);
                    let m1=scales[1]*3.*(d[3][k]-d[2][k]);
                    let expected=6.*t*q*(d[3][k]-d[0][k])+q*(1.-3.*t)*m0+t*(3.*t-2.)*m1;
                    assert!(mixed[k][0]<=expected && expected<=mixed[k][1]);
                }
            }
        }
        assert!(bridge.mixed_derivative_bounds([0.2,0.8],[0.,1.],1).unwrap().0.is_none());
    }
    #[test]
    fn anchored_hermite_enclosure_preserves_constant_coordinate_and_real_cubic() {
        let r=anchored_position_range([0.,1.],I::point(6.),I::point(6.),I::point(0.),I::new(0.,12.).unwrap()).unwrap();
        assert!(r.lo<=6. && r.hi>=6. && r.hi-r.lo<1e-10);
        assert!(anchored_position_range([-0.1,1.],I::point(1.),I::point(2.),I::point(0.),I::new(0.,3.).unwrap()).is_err());
        for range in [[0.,1.],[0.125,0.75],[0.5,0.5]] {
            let t=I::new(range[0],range[1]).unwrap();let q=I::point(1.).sub(t).unwrap();
            let tangent=t.mul(q).unwrap().mul(q).unwrap().mul(I::point(2.)).unwrap()
                .sub(t.mul(t).unwrap().mul(q).unwrap().mul(I::point(-3.)).unwrap()).unwrap();
            let r=anchored_position_range(range,I::point(3.),I::point(7.),tangent,I::new(-100.,100.).unwrap()).unwrap();
            for i in 0..=32 {
                let t=range[0]+(range[1]-range[0])*i as f64/32.;let q=1.-t;
                let expected=3.*q*q*(1.+2.*t)+7.*t*t*(3.-2.*t)+2.*t*q*q+3.*t*t*q;
                assert!(r.lo<=expected && expected<=r.hi,"{range:?}: {r:?} misses {expected}");
            }
        }
    }
    #[test]
    fn independent_profile_injectivity_distinguishes_plane_from_fold() {
        let plane=crate::surface::Surface {degree_u:1,degree_v:1,
            knots_u:vec![0.,0.,1.,1.],knots_v:vec![0.,0.,1.,1.],
            control_points:[0.,1.].into_iter().map(|u|vec![vec![u,0.,0.],vec![u,1.,0.]]).collect(),
            weights:vec![vec![1.;2];2],periodic_u:false,periodic_v:false};
        let other=plane.clone();
        let a=crate::normal_alignment::qualify_weight_profile(&plane,[0,0],100).unwrap().0.unwrap();
        let b=crate::normal_alignment::qualify_weight_profile(&other,[0,0],100).unwrap().0.unwrap();
        let projection=[[0.,1.,0.],[1.,0.,0.]];
        let bridge=ProfileSectionBridge::new(a.section(0,0.).unwrap(),b.section(0,1.).unwrap(),[1.;2]).unwrap();
        let admitted=bridge.inspect_injectivity([0.,1.],[0.,1.],projection,1000,10000).unwrap();
        assert!(admitted.complete_regular && admitted.pending.is_empty());
        // x(t)=4t-9t²+6t³. Its derivative at t=1/2 is -1/2;
        // t=1/2 and t=1/2 +/- sqrt(3)/6 all map to x=1/2.
        // These are distinct parameters of a genuine overlapping patch.
        let folded=ProfileSectionBridge::new(a.section(0,0.).unwrap(),b.section(0,1.).unwrap(),[4.;2]).unwrap();
        for cells in [0,1,7,100] {for spans in [0,1,100,10000] {
            let report=folded.inspect_injectivity([0.,1.],[0.,1.],projection,cells,spans).unwrap();
            assert!(!report.complete_regular);assert!(report.cells<=cells && report.spans<=spans);
            assert!(report.pending.iter().any(|r|r[0][0]<=0.5 && 0.5<=r[0][1] && r[1][0]<=0.5 && 0.5<=r[1][1]));
            let area:f64=report.regular.iter().chain(&report.pending)
                .map(|r|(r[0][1]-r[0][0])*(r[1][1]-r[1][0])).sum();
            assert_eq!(area,1.,"No unsupported product may disappear on exhaustion");
        }}
    }
    #[test]
    fn full_corrected_injectivity_refuses_a_genuinely_folded_patch() {
        // x(u)=(u-0.625)^2: distinct u values in [0.5,0.75]
        // coincide. The cubic bridge reproduces this quadratic exactly.
        let surface = crate::surface::Surface {
            degree_u: 2,
            degree_v: 1,
            knots_u: vec![0., 0., 0., 1., 1., 1.],
            knots_v: vec![0., 0., 1., 1.],
            control_points: [0.390625, -0.234375, 0.140625]
                .into_iter()
                .map(|x| vec![vec![x, 0., 0.], vec![x, 1., 0.]])
                .collect(),
            weights: vec![vec![1.; 2]; 3],
            periodic_u: false,
            periodic_v: false,
        };
        let profile = crate::normal_alignment::qualify_weight_profile(&surface, [0, 0], 6)
            .unwrap()
            .0
            .unwrap();
        let bridge = ProfileSectionBridge::new(
            profile.section(0, 0.5).unwrap(),
            profile.section(0, 0.75).unwrap(),
            [0.25; 2],
        )
        .unwrap();
        let corrected = bridge.constrain_original_rails(100).unwrap().0.unwrap();
        let report = corrected
            .inspect_transverse_corrected_injectivity_from_rails(
                [0., 1.],
                [0., 1.],
                [[0., 1., 0.], [1., 0., 0.]],
                20,
                1220,
            )
            .unwrap();
        assert!(!report.complete_regular);
        assert!(!report.pending.is_empty());
        let area: f64 = report
            .regular
            .iter()
            .chain(&report.pending)
            .map(|r| (r[0][1] - r[0][0]) * (r[1][1] - r[1][0]))
            .sum();
        assert_eq!(area, 1.);
        // Cell and derivative exhaustion are independent stop conditions.
        // Every stop must retain all uncovered regions of this real fold.
        for cells in [0,1,2,7] {
            for spans in [0,1,61,1220] {
                let stopped=corrected.inspect_transverse_corrected_injectivity_from_rails(
                    [0.,1.],[0.,1.],[[0.,1.,0.],[1.,0.,0.]],cells,spans).unwrap();
                assert!(!stopped.complete_regular);
                assert!(stopped.cells<=cells && stopped.spans<=spans);
                let area:f64=stopped.regular.iter().chain(&stopped.pending)
                    .map(|r|(r[0][1]-r[0][0])*(r[1][1]-r[1][0])).sum();
                assert_eq!(area,1.);
            }
        }
    }
    #[test]
    fn common_source_difference_tightens_translated_bridge_jet() {
        let surface = crate::surface::Surface {
            degree_u: 1,
            degree_v: 1,
            knots_u: vec![0., 0., 1., 1.],
            knots_v: vec![0., 0., 1., 1.],
            control_points: vec![
                vec![vec![1e8, 0., 0.], vec![1e8, 1., 0.]],
                vec![vec![1e8 + 1., 0., 0.], vec![1e8 + 1., 1., 0.]],
            ],
            weights: vec![vec![1.; 2]; 2],
            periodic_u: false,
            periodic_v: false,
        };
        let profile = crate::normal_alignment::qualify_weight_profile(&surface, [0, 0], 4)
            .unwrap()
            .0
            .unwrap();
        let bridge = ProfileSectionBridge::new(
            profile.section(0, 0.5).unwrap(),
            profile.section(0, 0.75).unwrap(),
            [0.25; 2],
        )
        .unwrap();
        let old = bridge
            .derivative_bounds([0.25, 0.75], [0.25, 0.75], 8)
            .unwrap()
            .0
            .unwrap();
        let (new, spent) = bridge
            .derivative_bounds_impl([0.25, 0.75], [0.25, 0.75], 9, true)
            .unwrap();
        let new = new.unwrap();
        assert_eq!(spent, 9);
        assert!(new[1][0][0] <= 0.25 && new[1][0][1] >= 0.25);
        assert!(new[1][0][1] - new[1][0][0] < old[1][0][1] - old[1][0][0]);
        assert!(
            bridge
                .derivative_bounds_impl([0.25, 0.75], [0.25, 0.75], 8, true)
                .unwrap()
                .0
                .is_none()
        );
    }
    #[test]
    fn principal_minors_admit_nonsymmetric_maps_and_refuse_orientation_failures() {
        let matrix = |m: [[f64; 2]; 2]| m.map(|row| row.map(I::point));
        // Positive principal minors, but indefinite symmetric part.
        assert!(projected_principal_minors_positive(matrix([[1., 8.], [-1., 1.]])).unwrap());
        assert!(!projected_principal_minors_positive(matrix([[-1., 0.], [0., -1.]])).unwrap());
        assert!(!projected_principal_minors_positive(matrix([[1., 2.], [2., 1.]])).unwrap());
        let uncertain = [
            [I::new(-0.1, 1.).unwrap(), I::point(0.)],
            [I::point(0.), I::point(1.)],
        ];
        assert!(!projected_principal_minors_positive(uncertain).unwrap());
    }
}

#[path="profile_section_bridge_second.rs"]
mod second_free_jet;
