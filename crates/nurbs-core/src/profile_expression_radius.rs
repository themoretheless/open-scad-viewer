//! Complete radius-law coverage of the exact corrected Hermite expression.
//! Reports carry no B-rep admission or caller-created certificate authority.
use crate::{
    check,
    curve::Curve,
    interval_eval::{self, Interval as I},
    profile_section_bridge::RailConstrainedBridge,
    Result,
};
pub use crate::profile_expression_bernstein::CoefficientRadiusBound;
pub use crate::profile_expression_bernstein::CoefficientLineReport;
pub struct Report {
    pub domain: [[f64; 2]; 2],
    pub complete: bool,
    pub cells: usize,
    pub spans: usize,
    pub accepted_cells: usize,
    pub accepted_error_upper: f64,
    pub pending: Vec<[[f64; 2]; 2]>,
    pub reason: &'static str,
    pub violation: Option<Violation>,
}
impl Report {
    /// Diagnostic parameter-area fraction, not physical surface area or proof.
    /// Admission continues to require complete coverage with no pending cells.
    pub fn pending_parameter_fraction(&self) -> f64 {
        self.pending
            .iter()
            .map(|rect| {
                (0..2)
                    .map(|axis| {
                        (rect[axis][1] - rect[axis][0])
                            / (self.domain[axis][1] - self.domain[axis][0])
                    })
                    .product::<f64>()
            })
            .sum::<f64>()
            .clamp(0., 1.)
    }
}
pub struct Violation {
    pub domain: [[f64; 2]; 2],
    pub error_bounds: [f64; 2],
    pub squared_residual: [f64; 2],
}
fn mapped(d: [f64; 2], across: [f64; 2]) -> Result<I> {
    I::point(d[0])
        .add(I::new(across[0], across[1])?.mul(I::point(d[1]).sub(I::point(d[0]))?)?)?
        .intersect(d[0], d[1])
}
fn spans(curve: &Curve, t: I) -> usize {
    (curve.degree..curve.control_points.len())
        .filter(|&i| {
            curve.knots[i] < curve.knots[i + 1]
                && curve.knots[i] <= t.hi
                && curve.knots[i + 1] >= t.lo
        })
        .count()
}
fn continuous_knots(knots: &[f64], degree: usize, count: usize, domain: [f64; 2]) -> bool {
    !knots[degree + 1..count].iter().any(|&knot| {
        domain[0] <= knot
            && knot <= domain[1]
            && knots.iter().filter(|&&k| k == knot).count() > degree
    })
}

#[cfg(test)]
#[path="profile_expression_radius/tests/profile_self_chord_tests.rs"]
mod profile_self_chord_tests;
// Original rational jets differentiate the restricted local [0,1] chart.
// Convert to source parameter, then to normalized bridge across parameter.
fn law_derivative(
    curve: &Curve,
    t: I,
    law_domain: [f64; 2],
    budget: usize,
) -> Result<(Option<Vec<I>>, usize)> {
    let mut spent = 0;
    let mut hull: Option<Vec<I>> = None;
    let scale = I::point(law_domain[1]).sub(I::point(law_domain[0]))?;
    for span in curve.degree..curve.control_points.len() {
        let lo = t.lo.max(curve.knots[span]);
        let hi = t.hi.min(curve.knots[span + 1]);
        if lo >= hi {
            continue;
        }
        if spent == budget {
            return Ok((None, spent));
        }
        spent += 1;
        let jet = crate::curve_jets::enclose(curve, span, [lo, hi])?;
        let width = I::point(hi).sub(I::point(lo))?;
        let derivative = jet[1]
            .iter()
            .map(|d| d.div(width)?.mul(scale))
            .collect::<Result<Vec<_>>>()?;
        if let Some(previous) = &mut hull {
            for (a, b) in previous.iter_mut().zip(derivative) {
                *a = I::new(a.lo.min(b.lo), a.hi.max(b.hi))?;
            }
        } else {
            hull = Some(derivative);
        }
    }
    Ok((hull, spent))
}
// Public workspaces retain the immutable bridge, and thus both section
// profiles, for this entire cache lifetime. Equal UV coordinates on its two
// distinct profiles do not identify equal geometry; include profile identity.
#[derive(Default)]
struct ProfileDerivativeCache {
    side_remainders: crate::profile_section_bridge::SideRemainderCache,
    entries:std::collections::HashMap<(usize,[[u64;2];2]),[[[f64;2];2];3]>,
    corrected:std::collections::HashMap<[[u64;2];2],[[[f64;2];3];2]>,
    directions:std::collections::HashMap<(usize,[[u64;2];2]),[[f64;2];3]>,
    positions:std::collections::HashMap<(usize,[[u64;2];2]),[[f64;2];3]>,
    corrected_positions:std::collections::HashMap<[[u64;2];2],[[f64;2];3]>,
    hits:usize,
}
impl ProfileDerivativeCache {
    fn enclose(&mut self,profile:&crate::surface_weight_profile::WeightProfile<'_>,domain:[[f64;2];2],budget:usize)
        ->Result<(Option<[[[f64;2];2];3]>,usize)> {
        let key=(profile as *const _ as usize,domain.map(|d|d.map(f64::to_bits)));
        if let Some(value)=self.entries.get(&key) {self.hits+=1;return Ok((Some(*value),0));}
        let (value,spent)=profile.derivative_bounds(domain,budget)?;
        if let Some(value)=value {if self.entries.len()<100000 {self.entries.insert(key,value);}}
        Ok((value,spent))
    }
    fn corrected_jet(&mut self,bridge:&RailConstrainedBridge<'_, '_, '_, '_, '_>,domain:[[f64;2];2],budget:usize)
        ->Result<(Option<[[[f64;2];3];2]>,usize)> {
        let key=domain.map(|d|d.map(f64::to_bits));
        if let Some(value)=self.corrected.get(&key) {self.hits+=1;return Ok((Some(*value),0));}
        let (value,spent)=bridge.derivatives_with_transverse_correction_cached(domain[0],domain[1],budget,&mut self.side_remainders)?;
        if let Some(value)=value {if self.corrected.len()<100000 {self.corrected.insert(key,value);}}
        Ok((value,spent))
    }
    fn profile_direction(&mut self,profile:&crate::surface_weight_profile::WeightProfile<'_>,domain:[[f64;2];2],budget:usize)
        ->Result<(Option<[[f64;2];3]>,usize)> {
        let key=(profile as *const _ as usize,domain.map(|d|d.map(f64::to_bits)));
        if let Some(value)=self.directions.get(&key) {self.hits+=1;return Ok((Some(*value),0));}
        let (value,spent)=profile.interior_normal_direction_bounds(domain,budget)?;
        // A zero-containing direction box can reflect an exhausted optional
        // pole-factor attempt. Do not freeze it before a larger-budget retry.
        if let Some(value)=value {if value.iter().any(|d|d[0]>0. || d[1]<0.)
            && self.directions.len()<100000 {self.directions.insert(key,value);}}
        Ok((value,spent))
    }
    fn position(&mut self,profile:&crate::surface_weight_profile::WeightProfile<'_>,domain:[[f64;2];2],budget:usize)
        ->Result<(Option<[[f64;2];3]>,usize)> {
        let key=(profile as *const _ as usize,domain.map(|d|d.map(f64::to_bits)));
        if let Some(value)=self.positions.get(&key) {self.hits+=1;return Ok((Some(*value),0));}
        let (value,spent)=profile.enclose(domain,budget)?;
        if let Some(value)=value {if self.positions.len()<100000 {self.positions.insert(key,value);}}
        Ok((value,spent))
    }
    fn corrected_position(&mut self,bridge:&RailConstrainedBridge<'_, '_, '_, '_, '_>,domain:[[f64;2];2],budget:usize)
        ->Result<(Option<[[f64;2];3]>,usize)> {
        let key=domain.map(|d|d.map(f64::to_bits));
        if let Some(value)=self.corrected_positions.get(&key) {self.hits+=1;return Ok((Some(*value),0));}
        let (value,spent)=bridge.enclose_with_transverse_correction_cached(domain[0],domain[1],budget,Some(&self.side_remainders))?;
        if let Some(value)=value {if self.corrected_positions.len()<100000 {self.corrected_positions.insert(key,value);}}
        Ok((value,spent))
    }
    fn corrected_normal(&mut self,bridge:&RailConstrainedBridge<'_, '_, '_, '_, '_>,domain:[[f64;2];2],budget:usize)
        ->Result<(Option<[[f64;2];3]>,usize)> {
        let (jet,spent)=self.corrected_jet(bridge,domain,budget)?;
        let Some([a,b])=jet else {return Ok((None,spent));};
        let mut bounds=[[0.;2];3];
        for k in 0..3 {
            let i=(k+1)%3;let j=(k+2)%3;
            let n=I::new(a[i][0],a[i][1])?.mul(I::new(b[j][0],b[j][1])?)?
                .sub(I::new(a[j][0],a[j][1])?.mul(I::new(b[i][0],b[i][1])?)?)?;
            bounds[k]=[n.lo,n.hi];
        }
        Ok((Some(bounds),spent))
    }
}
/// Invocation-local complete hull cache bound to one immutable retained profile.
/// UV keys cannot be shared with another profile; incomplete hulls are not stored.
pub struct WeightChordWorkspace<'p,'s> {
    profile:&'p crate::surface_weight_profile::WeightProfile<'s>,
    derivatives:ProfileDerivativeCache,
}
impl WeightChordWorkspace<'_, '_> {
    pub fn cache_hits(&self)->usize {self.derivatives.hits}
    /// Whole-chart exclusion shares complete jets with endpoint-pair checks.
    /// Domain validation and zero-budget refusal precede every cache lookup.
    pub fn inspect_self_chords(&mut self,domain:[[f64;2];2],sine:f64,max_spans:usize)
        ->Result<(Option<bool>,usize)> {
        self.profile.inspect_self_chords_cached(domain,sine,max_spans,&mut self.derivatives)
    }
    pub fn inspect_pair(&mut self,pair:[[[f64;2];2];2],sine:f64,max_spans:usize)
        ->Result<(Option<bool>,usize)> {
        self.profile.inspect_self_chord_pair_cached(pair,sine,max_spans,&mut self.derivatives)
    }
}
impl<'s> crate::surface_weight_profile::WeightProfile<'s> {
    pub fn chord_workspace(&self)->WeightChordWorkspace<'_,'s> {
        WeightChordWorkspace {profile:self,derivatives:ProfileDerivativeCache::default()}
    }
}
/// Invocation-local cache bound to an immutable native corrected transition.
/// Complete profile/corrected jets, positions and directions only; no caller-supplied bounds or wire admission.
pub struct ProfileChordWorkspace<'x,'r,'a,'b,'c,'d> {
    bridge:&'x RailConstrainedBridge<'r,'a,'b,'c,'d>,
    derivatives:ProfileDerivativeCache,
}
impl ProfileChordWorkspace<'_, '_, '_, '_, '_, '_> {
    pub fn cache_hits(&self)->usize {self.derivatives.hits}
    /// Whole-rectangle exclusion using the same complete owner-bound jets
    /// as endpoint-pair queries. Cached hulls do not bypass zero-work refusal.
    pub fn inspect_self_chords(&mut self,domain:[[f64;2];2],sine:f64,max_spans:usize)
        ->Result<(Option<bool>,usize)> {
        self.bridge.inspect_self_chords_cached(domain,sine,max_spans,&mut self.derivatives)
    }
    /// Complete owner-bound position hull shared with endpoint-pair queries.
    /// Exhaustion and invalid rectangles never borrow an earlier admission.
    pub fn enclose_position(&mut self,domain:[[f64;2];2],max_spans:usize)
        ->Result<(Option<[[f64;2];3]>,usize)> {
        check(max_spans<=100000,"Bound corrected position cache work")?;
        self.bridge.enclose_with_transverse_correction(domain[0],domain[1],0)?;
        if max_spans==0 {return Ok((None,0));}
        self.derivatives.corrected_position(self.bridge,domain,max_spans)
    }
    /// Complete interval derivatives on this workspace's immutable owner.
    pub fn enclose_derivatives(&mut self,domain:[[f64;2];2],max_spans:usize)
        ->Result<(Option<[[[f64;2];3];2]>,usize)> {
        check(max_spans<=100000,"Bound corrected derivative cache work")?;
        self.bridge.derivatives_with_transverse_correction(domain[0],domain[1],0)?;
        if max_spans==0 {return Ok((None,0));}
        self.derivatives.corrected_jet(self.bridge,domain,max_spans)
    }
    pub fn inspect_corrected_pair(&mut self,pair:[[[f64;2];2];2],sine:f64,max_spans:usize)
        ->Result<(Option<bool>,usize)> {
        self.bridge.inspect_self_chord_pair_cached(pair,sine,max_spans,&mut self.derivatives)
    }
    pub fn inspect(&mut self,profile_domain:[[f64;2];2],bridge_domain:[[f64;2];2],
        high:bool,sine:f64,max_spans:usize)->Result<(Option<bool>,usize)> {
        self.bridge.inspect_profile_endpoint_chord_cached(profile_domain,bridge_domain,high,sine,max_spans,&mut self.derivatives)
    }
}
impl<'r,'a,'b,'c,'d> RailConstrainedBridge<'r,'a,'b,'c,'d> {
    pub fn profile_chord_workspace(&self)->ProfileChordWorkspace<'_,'r,'a,'b,'c,'d> {
        ProfileChordWorkspace {bridge:self,derivatives:ProfileDerivativeCache::default()}
    }
}

#[cfg(test)]
#[path = "profile_expression_radius/tests/mod.rs"]
mod tests;

#[path="profile_expression_radius/methods_crate_surface_weight_profile_WeightProfile____inspect_self_chord_pair.rs"]
mod methods_crate_surface_weight_profile_weight_profile__inspect_self_chord_pair;
#[path="profile_expression_radius/methods_RailConstrainedBridge____________self_chord_continuous.rs"]
mod methods_rail_constrained_bridge______self_chord_continuous;
#[path="profile_expression_radius/methods_RailConstrainedBridge____________inspect_radius_on_domains.rs"]
mod methods_rail_constrained_bridge______inspect_radius_on_domains;
