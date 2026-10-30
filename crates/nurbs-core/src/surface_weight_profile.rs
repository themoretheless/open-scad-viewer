//! Positional error bound for an exact separable original rational weight profile.
use crate::{Result, check, interval_eval::Interval as I, surface::Surface};
// Compare positive binary64 products as exact dyadic integers. Each significand
// has at most 53 bits, so their product fits u128 even when f64 would overflow
// or underflow. Surface validation guarantees finite positive weights.
fn exact_positive_products_equal(a:f64,b:f64,c:f64,d:f64)->bool {
    fn product(a:f64,b:f64)->(u128,i32) {
        fn parts(x:f64)->(u128,i32) {
            let bits=x.to_bits();let exponent=((bits>>52)&0x7ff) as i32;
            let fraction=bits&((1_u64<<52)-1);
            if exponent==0 {(fraction as u128,-1074)}
            else {(((1_u64<<52)|fraction) as u128,exponent-1023-52)}
        }
        let (am,ae)=parts(a);let (bm,be)=parts(b);let m=am*bm;
        let shift=m.trailing_zeros();(m>>shift,ae+be+shift as i32)
    }
    [a,b,c,d].iter().all(|v|v.is_finite()&&*v>0.) && product(a,b)==product(c,d)
}
/// A proposal certificate, not a replacement Surface or a B-rep admission.
/// The target weights are the exact expressions row[i]*column[j]/pivot;
/// flattening those expressions to binary64 is not covered by this certificate.
pub struct WeightProfile<'a> {
    source: &'a Surface,
    row: Vec<f64>,
    column: Vec<f64>,
    pivot: f64,
    axis_deviation_upper: [f64; 3],
    weight_deviation_upper: f64,
}
/// Explicit collapsed-boundary normal factor diagnostic, never a smooth pole certificate.
pub struct ProfilePoleNormal {pub components:[[f64;2];3],pub high:bool,pub power:usize,pub point:[f64;3]}
/// Coverage of a factored normal, explicitly distinct from pole immersion.
pub struct ProfilePoleCover {
    pub regular:Vec<[[f64;2];2]>,pub pending:Vec<[[f64;2];2]>,pub cells:usize,pub spans:usize,
    pub complete_factored:bool,pub high:bool,pub power:Option<usize>,pub point:Option<[f64;3]>,pub reason:&'static str,
}
/// Direction certificate for the exact proposed profile, not its original Surface.
pub struct ProfileDirection<'a, 's> {
    profile: &'a WeightProfile<'s>,
    endpoints: [[f64; 2]; 2],
    axis: usize,
    boundary: f64,
    factor: Option<(bool, usize)>,
    components: [[f64; 2]; 3],
}
impl ProfileDirection<'_, '_> {
    pub fn profile(&self) -> &WeightProfile<'_> {
        self.profile
    }
    pub fn endpoints(&self) -> [[f64; 2]; 2] {
        self.endpoints
    }
    pub fn axis(&self) -> usize {
        self.axis
    }
    pub fn boundary(&self) -> f64 {
        self.boundary
    }
    pub fn factor(&self) -> Option<(bool, usize)> {
        self.factor
    }
    pub fn components(&self) -> [[f64; 2]; 3] {
        self.components
    }
}
/// Pointwise equality of one original natural boundary and the proposal.
pub struct ProfileBoundary<'a, 's> {
    profile: &'a WeightProfile<'s>,
    axis: usize,
    boundary: f64,
}
impl ProfileBoundary<'_, '_> {
    pub fn profile(&self) -> &WeightProfile<'_> {
        self.profile
    }
    pub fn axis(&self) -> usize {
        self.axis
    }
    pub fn boundary(&self) -> f64 {
        self.boundary
    }
}
/// Complete angular coverage of two affine parameter lines. This does not
/// establish that the lines are the same B-rep edge.
pub struct ProfilePathReport {
    pub aligned: Option<bool>,
    pub cells: usize,
    pub spans: usize,
    pub accepted_cells: usize,
    pub upper_sine_squared: Option<f64>,
    pub pending: Vec<[f64; 2]>,
    pub reason: &'static str,
}
/// Diagnostic limiting planes for a collapsed natural U boundary of the exact
/// proposed profile. Distinct limits cannot authorize a regular G1 pole.
pub struct ProfilePoleLimits {
    pub point: [f64; 3],
    pub components: [[[f64; 2]; 3]; 2],
    pub sine_squared_interval: Option<[f64; 2]>,
    pub distinct_tangent_limits: bool,
}
/// Exact iso-parameter cut carrier on one immutable proposed profile.
/// This view keeps source/factor ownership instead of flattening new weights.
pub struct ProfileSection<'p, 's> {
    profile: &'p WeightProfile<'s>,
    axis: usize,
    parameter: f64,
    range: [f64; 2],
}

/// Exact cubic Hermite end-transition expression attached to one immutable
/// profile section. Side rails and the collapsed tip are not B-rep admissions.
pub struct ProfileEndTransition<'p, 's> {
    section: ProfileSection<'p, 's>,
    target: [f64; 3],
    transverse_scale: f64,
}
impl<'p, 's> ProfileSection<'p, 's> {
    pub fn end_transition(
        self,
        target: [f64; 3],
        transverse_scale: f64,
    ) -> Result<ProfileEndTransition<'p, 's>> {
        check(
            target.iter().all(|x| x.is_finite())
                && transverse_scale.is_finite()
                && transverse_scale != 0.,
            "Require a finite target and nonzero transverse scale",
        )?;
        Ok(ProfileEndTransition {
            section: self,
            target,
            transverse_scale,
        })
    }
}

pub struct TransitionRegularityReport {
    pub regular: Vec<[[f64; 2]; 2]>,
    pub pending: Vec<[[f64; 2]; 2]>,
    pub cells: usize,
    pub spans: usize,
    pub complete_regular: bool,
    pub reason: &'static str,
}
/// Original side-rail expression bound to the exact transition. It cannot be
/// substituted by another curve merely because endpoint coordinates overlap.
pub struct ProfileTransitionRail<'a, 'p, 's> {
    transition: &'a ProfileEndTransition<'p, 's>,
    free: f64,
}
impl ProfileTransitionRail<'_, '_, '_> {
    pub fn free_parameter(&self) -> f64 {
        self.free
    }
    pub fn transition(&self) -> &ProfileEndTransition<'_, '_> {
        self.transition
    }
    pub fn enclose(
        &self,
        range: [f64; 2],
        max_spans: usize,
    ) -> Result<(Option<[[f64; 2]; 3]>, usize)> {
        self.transition.enclose([self.free; 2], range, max_spans)
    }
    pub fn tangent_bounds(
        &self,
        range: [f64; 2],
        max_spans: usize,
    ) -> Result<(Option<[[f64; 2]; 3]>, usize)> {
        self.transition
            .across_derivative_bounds([self.free; 2], range, max_spans)
    }
}
fn choose(n: usize, k: usize) -> f64 {
    let mut x = 1u64;
    for i in 0..k.min(n - k) {
        x = x * (n - i) as u64 / (i + 1) as u64;
    }
    x as f64
}

/// Cover every original control. Whole-domain positional deviation follows from
/// positive rational basis weights, without sampling or derivative assumptions.
pub fn qualify_weight_profile<'a>(
    s: &'a Surface,
    anchor: [usize; 2],
    max_controls: usize,
) -> Result<(Option<WeightProfile<'a>>, usize)> {
    s.validate()?;
    check(
        max_controls <= 1_000_000 && anchor[0] < s.weights.len() && anchor[1] < s.weights[0].len(),
        "Choose an original profile anchor and bounded control work",
    )?;
    if s.periodic_u || s.periodic_v {
        return Ok((None, 0));
    }
    let row: Vec<_> = s.weights.iter().map(|r| r[anchor[1]]).collect();
    let column = s.weights[anchor[0]].clone();
    let pivot = s.weights[anchor[0]][anchor[1]];
    let mut delta = 0_f64;
    let mut lower = f64::INFINITY;
    let mut ranges = [[f64::INFINITY, f64::NEG_INFINITY]; 3];
    let mut spent = 0;
    for i in 0..row.len() {
        for j in 0..column.len() {
            if spent == max_controls {
                return Ok((None, spent));
            }
            let target = I::point(row[i])
                .mul(I::point(column[j]))?
                .div(I::point(pivot))?;
            let difference = target.sub(I::point(s.weights[i][j]))?;
            delta = delta.max(difference.lo.abs()).max(difference.hi.abs());
            lower = lower.min(target.lo);
            for k in 0..3 {
                let coordinate = s.control_points[i][j][k];
                ranges[k][0] = ranges[k][0].min(coordinate);
                ranges[k][1] = ranges[k][1].max(coordinate);
            }
            spent += 1;
        }
    }
    if lower <= 0. {
        return Ok((None, spent));
    }
    // T-S = sum B_ij (targetWeight-originalWeight)(P_ij-S)/targetW.
    // Positive original weights place S in the original control convex hull;
    // sum B_ij=1 and targetW>=min(targetWeight). No chart samples enter proof.
    let mut axis_deviation_upper = [0.; 3];
    for k in 0..3 {
        let extent = I::point(ranges[k][1]).sub(I::point(ranges[k][0]))?;
        axis_deviation_upper[k] = extent.mul(I::point(delta))?.div(I::point(lower))?.hi;
    }
    Ok((
        Some(WeightProfile {
            source: s,
            row,
            column,
            pivot,
            axis_deviation_upper,
            weight_deviation_upper: delta,
        }),
        spent,
    ))
}
#[cfg(test)]
#[path = "surface_weight_profile/tests/mod.rs"]
mod tests;

#[path="surface_weight_profile/methods_ProfileSection______profile.rs"]
mod methods_ProfileSection______profile;
#[path="surface_weight_profile/methods_ProfileEndTransition______section.rs"]
mod methods_ProfileEndTransition______section;
#[path="surface_weight_profile/methods_WeightProfile_source__section.rs"]
mod methods_WeightProfile_source__section;
#[path="surface_weight_profile/methods_WeightProfile_source__inspect_profile_path.rs"]
mod methods_WeightProfile_source__inspect_profile_path;
