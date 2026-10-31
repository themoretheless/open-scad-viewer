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

#[cfg(test)]
#[path = "profile_section_bridge/tests/mod.rs"]
mod tests;

#[path="profile_section_bridge_second.rs"]
mod second_free_jet;

#[path="profile_section_bridge/methods_ProfileSectionBridge_a_b_c_d__new.rs"]
mod methods_profile_section_bridge_a_b_c_d_new;
#[path="profile_section_bridge/methods_ProfileSectionBridge_a_b_c_d__inspect_injectivity.rs"]
mod methods_profile_section_bridge_a_b_c_d_inspect_injectivity;
#[path="profile_section_bridge/methods_RailConstrainedBridge____________section.rs"]
mod methods_rail_constrained_bridge______section;
#[path="profile_section_bridge/methods_RailConstrainedBridge____________inspect_transverse_corrected_injectivity_policy.rs"]
mod methods_rail_constrained_bridge______inspect_transverse_corrected_injectivity_policy;
