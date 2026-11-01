//! Original filled embedding with Ck motion and explicitly classified traces.
//! This owner does not assert that source profiles bound a valid material trim.
use super::*;

#[derive(Clone, Copy)]
pub struct OriginalClassifiedVolumeLimits {
    /// One aggregate cell/exact/product budget for charts AND profile jets.
    pub charts: OriginalSmoothVolumeLimits,
    pub steps: usize,
}
#[derive(Debug)]
pub struct OriginalClassifiedVolumeReport {
    pub certified: bool,
    /// Closed motion may be certified while uniform profile smoothness is false.
    pub charts: OriginalSmoothVolumeReport,
    pub profiles: Option<OriginalProfileBoundaryReport>,
    pub cells: usize,
    pub exact_work: u64,
    pub products: usize,
    pub reason: Option<&'static str>,
}
/// A distinct opaque owner: sharp declared C0 joins never acquire uniform Gk.
/// A material owner must additionally bind every original trace to its trims
/// before transporting these geometric joins to a regular boundary surface.
pub struct OriginalClassifiedVolume<'s, 'a> {
    sweep: &'s MultiSweep<'a>,
    caps: [&'s Surface; 2],
    loop_sizes: Vec<usize>,
    classes: Vec<OriginalProfileClass>,
    report: OriginalClassifiedVolumeReport,
}
impl<'s, 'a> OriginalClassifiedVolume<'s, 'a> {
    pub fn sweep(&self) -> &'s MultiSweep<'a> {
        self.sweep
    }
    pub fn caps(&self) -> [&'s Surface; 2] {
        self.caps
    }
    pub fn loop_sizes(&self) -> &[usize] {
        &self.loop_sizes
    }
    pub fn classes(&self) -> &[OriginalProfileClass] {
        &self.classes
    }
    pub fn report(&self) -> &OriginalClassifiedVolumeReport {
        &self.report
    }
}
pub enum OriginalClassifiedVolumeAttempt<'s, 'a> {
    Certified(Box<OriginalClassifiedVolume<'s, 'a>>),
    Refused(Box<OriginalClassifiedVolumeReport>),
}
impl<'a> MultiSweep<'a> {
    /// Fresh full-volume embedding, cap Ck, periodic/open motion Ck and exact
    /// per-span source profile joins. No sampled or caller-authored proof input.
    /// Classes include all internal knots and ring closure in source order.
    pub fn certify_original_classified_volume_atlas<'s>(
        &'s self,
        caps: [&'s Surface; 2],
        loop_sizes: &[usize],
        classes: &[OriginalProfileClass],
        projection: [[f64; 3]; 3],
        limits: OriginalClassifiedVolumeLimits,
    ) -> Result<OriginalClassifiedVolumeAttempt<'s, 'a>> {
        check(
            !classes.is_empty() && classes.len() <= 1000000 && limits.steps <= 1000000,
            "Invalid classified original volume work",
        )?;
        let charts = super::original_smooth_volume::certify_charts(
            self,
            caps,
            loop_sizes,
            projection,
            limits.charts,
            false,
        )?;
        let mut out = OriginalClassifiedVolumeReport {
            certified: false,
            cells: charts.cells,
            exact_work: charts.exact_work,
            products: charts.products,
            reason: charts.reason,
            charts,
            profiles: None,
        };
        if !out.charts.certified {
            return Ok(OriginalClassifiedVolumeAttempt::Refused(Box::new(out)));
        }
        let profiles = super::original_profile_boundary::inspect_profiles(
            self,
            loop_sizes,
            limits.charts.order,
            classes,
            OriginalProfileBoundaryLimits {
                steps: limits.steps,
                cells: limits.charts.cells - out.cells,
                exact_work: limits.charts.exact_work - out.exact_work,
                products: limits.charts.products - out.products,
            },
        )?;
        out.cells += profiles.cells;
        out.exact_work += profiles.exact_work;
        out.products += profiles.products;
        out.reason = profiles.reason;
        out.certified = profiles.certified;
        out.profiles = Some(profiles);
        if !out.certified {
            return Ok(OriginalClassifiedVolumeAttempt::Refused(Box::new(out)));
        }
        Ok(OriginalClassifiedVolumeAttempt::Certified(Box::new(
            OriginalClassifiedVolume {
                sweep: self,
                caps,
                loop_sizes: loop_sizes.to_vec(),
                classes: classes.to_vec(),
                report: out,
            },
        )))
    }
}
#[cfg(test)]
mod tests;
