//! surface weight profile operations: profile.
use super::*;
impl ProfileSection<'_, '_> {
pub fn profile(&self) -> &WeightProfile<'_> {
        self.profile
    }
pub fn axis(&self) -> usize {
        self.axis
    }
pub fn parameter(&self) -> f64 {
        self.parameter
    }
pub fn range(&self) -> [f64; 2] {
        self.range
    }
pub(crate) fn rectangle(&self, range: [f64; 2]) -> Result<[[f64; 2]; 2]> {
        check(
            range.iter().all(|x| x.is_finite())
                && range[0] <= range[1]
                && range[0] >= self.range[0]
                && range[1] <= self.range[1],
            "Keep section parameters in its owned range",
        )?;
        let mut domain = [[self.parameter; 2]; 2];
        domain[1 - self.axis] = range;
        Ok(domain)
    }
pub fn enclose(
        &self,
        range: [f64; 2],
        max_spans: usize,
    ) -> Result<(Option<[[f64; 2]; 3]>, usize)> {
        self.profile.enclose(self.rectangle(range)?, max_spans)
    }
/// Complete derivative of the exact section with respect to its free
    /// original parameter, including both one-sided knot limits.
    pub fn tangent_bounds(
        &self,
        range: [f64; 2],
        max_spans: usize,
    ) -> Result<(Option<[[f64; 2]; 3]>, usize)> {
        let (bounds, spent) = self
            .profile
            .derivative_bounds(self.rectangle(range)?, max_spans)?;
        Ok((
            bounds.map(|b| std::array::from_fn(|k| b[k][1 - self.axis])),
            spent,
        ))
    }
/// Transverse support derivative for constructing a boundary-compatible
    /// transition. This does not certify any independently proposed cap.
    pub fn transverse_bounds(
        &self,
        range: [f64; 2],
        max_spans: usize,
    ) -> Result<(Option<[[f64; 2]; 3]>, usize)> {
        let (bounds, spent) = self
            .profile
            .derivative_bounds(self.rectangle(range)?, max_spans)?;
        Ok((
            bounds.map(|b| std::array::from_fn(|k| b[k][self.axis])),
            spent,
        ))
    }
/// Normals belong to the incident proposed support, not an as-yet unauthored cap.
    pub fn support_normals(
        &self,
        range: [f64; 2],
        max_spans: usize,
    ) -> Result<(Option<[[f64; 2]; 3]>, usize)> {
        self.profile
            .normal_bounds(self.rectangle(range)?, max_spans)
    }
}
