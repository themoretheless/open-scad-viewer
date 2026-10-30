//! profile section bridge operations: new.
use super::*;
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
pub(super) fn validate(&self, free: [f64; 2], across: [f64; 2], budget: usize) -> Result<()> {
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
pub(super) fn data(
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
pub(super) fn enclose_cached(
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
pub(super) fn derivative_bounds_impl(
        &self,
        free: [f64; 2],
        across: [f64; 2],
        budget: usize,
        common_difference: bool,
    ) -> Result<(Option<[[[f64; 2]; 3]; 2]>, usize)> {
        self.derivative_bounds_cached(free, across, budget, common_difference, None)
    }
pub(super) fn derivative_bounds_cached(
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
pub(super) fn chord_continuous(&self)->bool {
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
}
