//! Complete original helical patch census and inclusive nominal thread accuracy.
use super::*;
use crate::distance_bounds::{Interval as I, box_distance};
#[derive(Clone, Debug)]
pub struct ThreadCertificate {
    pub error_upper: f64,
    pub patches: usize,
    pub root_radius: [f64; 2],
    pub crest_radius: [f64; 2],
    pub lead: [f64; 2],
    pub rounding_included: bool,
}
fn pair(x: I) -> [f64; 2] {
    [x.lo, x.hi]
}
/// Independently check all actual saved patches against the nominal sharp
/// truncated 60-degree thread, including radius, sqrt(3), lead, phase, start
/// offsets, radians and saved control-net rounding. No platform trig is trusted.
/// Complete source-role census is mandatory; patches cannot be omitted/relabelled.
pub fn certify_patches(
    spec: Spec,
    source: &Patches,
    max_patches: usize,
) -> Result<ThreadCertificate> {
    let p = parameters(spec)?;
    let count = 20 * spec.starts * spec.turns;
    check(
        (1..=2560).contains(&max_patches) && count <= max_patches && source.patches.len() == count,
        "Thread certificate needs the complete bounded original patch census",
    )?;
    check(
        source.crest_radius == p.crest
            && source.root_radius == p.root
            && source.lead == p.lead
            && source.axial_extent == [0., p.maximum_z],
        "Thread source metadata differs from its original construction",
    )?;
    let diameter = I::point(spec.diameter);
    let pitch = I::point(spec.pitch);
    let allowance = I::point(spec.clearance).div(I::point(2.))?;
    let crest = if spec.kind == Kind::External {
        diameter.div(I::point(2.))?.sub(allowance)?
    } else {
        diameter.div(I::point(2.))?.add(allowance)?
    };
    let sqrt3 = I::new(3_f64.sqrt().next_down(), 3_f64.sqrt().next_up())?;
    let root = crest.sub(sqrt3.mul(pitch)?.mul(I::point(5.))?.div(I::point(16.))?)?;
    check(
        root.lo > 0. && root.hi < crest.lo,
        "Nominal thread radius ordering remains unresolved",
    )?;
    let lead = pitch.mul(I::point(spec.starts as f64))?;
    let direction = if spec.hand == Hand::Right { 1. } else { -1. };
    let quarter = crate::involute::certified_pi()?
        .div(I::point(2.))?
        .mul(I::point(direction))?;
    let phases = [0., 1. / 16., 3. / 8., 5. / 8., 15. / 16., 1.];
    let radii = [crest, crest, root, root, crest, crest];
    let stored_radii = [p.crest, p.crest, p.root, p.root, p.crest, p.crest];
    let mut maximum = 0_f64;
    for (index, patch) in source.patches.iter().enumerate() {
        let segment = index % 5;
        let q = (index / 5) % 4;
        let turn = (index / 20) % spec.turns;
        let start = index / (20 * spec.turns);
        check(
            (
                patch.start,
                patch.turn,
                patch.quarter,
                patch.profile_segment,
            ) == (start, turn, q, segment),
            "Thread patch role census differs from the nominal source",
        )?;
        check(
            patch.surface.control_points.len() == 2
                && patch
                    .surface
                    .control_points
                    .iter()
                    .all(|row| row.len() <= 31),
            "Thread certificate requires original bounded two-row patch storage",
        )?;
        let stored_z = start as f64 * spec.pitch + (turn as f64 + q as f64 / 4.) * p.lead;
        let z = I::point(start as f64)
            .mul(pitch)?
            .add(I::point(turn as f64 + q as f64 / 4.).mul(lead)?)?;
        let curve = Curve::from_polyline(
            (segment..=segment + 1)
                .map(|j| vec![stored_radii[j], 0., stored_z + phases[j] * spec.pitch])
                .collect(),
        )?;
        let mut profile_error = 0_f64;
        for (i, j) in (segment..=segment + 1).enumerate() {
            let actual: Vec<_> = curve.control_points[i]
                .iter()
                .map(|x| I::point(*x))
                .collect();
            let ideal = [
                radii[j],
                I::point(0.),
                z.add(I::point(phases[j]).mul(pitch)?)?,
            ];
            profile_error = profile_error.max(box_distance(&actual, &ideal)?.1);
        }
        let start_angle = quarter.mul(I::point(q as f64))?;
        let saved = helical_sweep::certify_stored(
            &curve,
            &patch.surface,
            pair(lead.div(I::point(4.))?),
            pair(start_angle),
            pair(quarter),
        )?;
        // Rotation preserves profile uncertainty; affine rise is already part
        // of the saved-wall family bound. Triangle inequality is outward.
        maximum = maximum.max(I::point(profile_error).add(I::point(saved.error_upper))?.hi);
    }
    Ok(ThreadCertificate {
        error_upper: maximum,
        patches: count,
        root_radius: pair(root),
        crest_radius: pair(crest),
        lead: pair(lead),
        rounding_included: true,
    })
}
/// Construct and certify complete thread patches. An unmet requested budget
/// refuses rather than returning an inaccurate "certified" result. The fast
/// unqualified candidate constructor remains available as `patches`.
pub fn patches_certified(spec: Spec, max_patches: usize) -> Result<(Patches, ThreadCertificate)> {
    parameters(spec)?;
    check(
        (1..=2560).contains(&max_patches) && 20 * spec.starts * spec.turns <= max_patches,
        "Thread certification patch budget is insufficient",
    )?;
    let mut source = patches(spec)?;
    let proof = certify_patches(spec, &source, max_patches)?;
    check(
        proof.error_upper <= spec.error_budget,
        "Inclusive nominal thread patch error exceeds its requested budget",
    )?;
    source.rounding_certified = true;
    Ok((source, proof))
}
