//! Exact declared original profile-join jets, anchored to a smooth volume.
//! Material trace ownership is still required before regular-wall Gk credit.
use super::*;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OriginalProfileClass {
    C0,
    G1,
    G2,
}
impl OriginalProfileClass {
    fn order(self) -> usize {
        match self {
            Self::C0 => 0,
            Self::G1 => 1,
            Self::G2 => 2,
        }
    }
}
#[derive(Clone, Copy)]
pub struct OriginalProfileBoundaryLimits {
    pub steps: usize,
    pub cells: usize,
    pub exact_work: u64,
    pub products: usize,
}
#[derive(Clone, Debug)]
pub struct OriginalProfileBoundaryReport {
    pub certified: bool,
    pub steps: usize,
    pub cells: usize,
    pub exact_work: u64,
    pub products: usize,
    pub inspected_joins: usize,
    pub smooth_joins: usize,
    pub uncertain_join: Option<[usize; 2]>,
    pub reason: Option<&'static str>,
}
/// Fresh exact compatibility for the actual immutable source profile packet.
/// Ck volume + this token is NOT a material boundary proof without UV trace
/// binding. No report field can construct or rebind the successful object.
pub struct OriginalProfileBoundary<'v, 's, 'a> {
    volume: &'v OriginalSmoothVolume<'s, 'a>,
    classes: Vec<OriginalProfileClass>,
    report: OriginalProfileBoundaryReport,
}
impl<'v, 's, 'a> OriginalProfileBoundary<'v, 's, 'a> {
    pub fn volume(&self) -> &'v OriginalSmoothVolume<'s, 'a> {
        self.volume
    }
    pub fn classes(&self) -> &[OriginalProfileClass] {
        &self.classes
    }
    pub fn report(&self) -> &OriginalProfileBoundaryReport {
        &self.report
    }
}
pub enum OriginalProfileBoundaryAttempt<'v, 's, 'a> {
    Certified(Box<OriginalProfileBoundary<'v, 's, 'a>>),
    Refused(Box<OriginalProfileBoundaryReport>),
}
fn refused(
    mut r: OriginalProfileBoundaryReport,
    reason: &'static str,
) -> OriginalProfileBoundaryReport {
    r.reason = Some(reason);
    r
}
impl<'s, 'a> OriginalSmoothVolume<'s, 'a> {
    /// Classes follow every exact Bezier span in source profile/loop order;
    /// entry i governs the forward join after that span, including ring closure.
    /// Every internal original knot is covered. C0 entries still require exact
    /// position closure. Smooth entries require original regular traces and
    /// positive geometric endpoint jet compatibility, never sampled normals.
    pub fn certify_profile_boundaries<'v>(
        &'v self,
        classes: &[OriginalProfileClass],
        l: OriginalProfileBoundaryLimits,
    ) -> Result<OriginalProfileBoundaryAttempt<'v, 's, 'a>> {
        let out = inspect_profiles(
            self.sweep(),
            self.loop_sizes(),
            self.report().order,
            classes,
            l,
        )?;
        if !out.certified {
            return Ok(OriginalProfileBoundaryAttempt::Refused(Box::new(out)));
        }
        Ok(OriginalProfileBoundaryAttempt::Certified(Box::new(
            OriginalProfileBoundary {
                volume: self,
                classes: classes.to_vec(),
                report: out,
            },
        )))
    }
}

/// Shared exact source census; report data alone confers no boundary authority.
pub(super) fn inspect_profiles(
    sweep: &MultiSweep<'_>,
    loop_sizes: &[usize],
    order: usize,
    classes: &[OriginalProfileClass],
    l: OriginalProfileBoundaryLimits,
) -> Result<OriginalProfileBoundaryReport> {
    check(
        !classes.is_empty()
            && classes.len() <= 1000000
            && l.steps <= 1000000
            && l.cells <= 1000000
            && l.exact_work <= 1000000
            && l.products <= 1000000,
        "Invalid declared original profile boundary work",
    )?;
    let mut out = OriginalProfileBoundaryReport {
        certified: false,
        steps: 0,
        cells: 0,
        exact_work: 0,
        products: 0,
        inspected_joins: 0,
        smooth_joins: 0,
        uncertain_join: None,
        reason: None,
    };
    if classes.iter().any(|c| c.order() > order) {
        return Ok(refused(
            out,
            "original-profile-boundary-volume-order-unproved",
        ));
    }
    let profiles = sweep.source_profiles();
    let mut offset = 0;
    let mut cursor = 0;
    for (ring, &count) in loop_sizes.iter().enumerate() {
        let mut pieces = Vec::new();
        for profile in &profiles[offset..offset + count] {
            if out.steps == l.steps || out.cells == l.cells {
                return Ok(refused(out, "original-profile-boundary-work-limit"));
            }
            out.steps += 1;
            let regular =
                crate::curve_regularity::inspect(profile, (l.cells - out.cells).min(100000))?;
            out.cells += regular.cells;
            if !regular.spanwise_regular {
                return Ok(refused(out, "original-profile-boundary-speed-unproved"));
            }
            if !(1..=32).contains(&profile.degree) || profile.control_points.len() > 4096 {
                return Ok(refused(out, "original-profile-boundary-basis-unproved"));
            }
            let rows = (profile.degree..profile.control_points.len())
                .filter(|&i| profile.knots[i] < profile.knots[i + 1])
                .count()
                * (profile.degree + 1);
            if rows > l.products - out.products {
                return Ok(refused(out, "original-profile-boundary-product-limit"));
            }
            out.products += rows;
            let Some(spans) =
                crate::retained_wall_coefficients::exact_bezier_controls(profile, rows.min(4096))
            else {
                return Ok(refused(
                    out,
                    "original-profile-boundary-exact-extraction-unproved",
                ));
            };
            pieces.extend(spans);
        }
        offset += count;
        for i in 0..pieces.len() {
            out.uncertain_join = Some([ring, i]);
            if out.steps == l.steps {
                return Ok(refused(out, "original-profile-boundary-work-limit"));
            }
            out.steps += 1;
            let Some(&class) = classes.get(cursor) else {
                return Ok(refused(
                    out,
                    "original-profile-boundary-class-count-mismatch",
                ));
            };
            cursor += 1;
            let (a, b) = (&pieces[i], &pieces[(i + 1) % pieces.len()]);
            // Exact source decomposition has proved these literal clamped
            // endpoints. Equal binary coordinates are exact position credit.
            if a.control_points.last() != b.control_points.first() {
                return Ok(refused(out, "original-profile-boundary-position-unproved"));
            }
            if class != OriginalProfileClass::C0 {
                if out.exact_work == l.exact_work {
                    return Ok(refused(out, "original-profile-boundary-work-limit"));
                }
                let (proved, work) = super::original_boundary_smoothness::profile_join(
                    a,
                    b,
                    class.order(),
                    l.exact_work - out.exact_work,
                )?;
                out.exact_work += work;
                if !proved {
                    return Ok(refused(
                        out,
                        "original-profile-boundary-geometric-join-unproved",
                    ));
                }
                out.smooth_joins += 1;
            }
            out.inspected_joins += 1;
            out.uncertain_join = None;
        }
    }
    if cursor != classes.len() {
        return Ok(refused(
            out,
            "original-profile-boundary-class-count-mismatch",
        ));
    }
    out.certified = true;
    Ok(out)
}
