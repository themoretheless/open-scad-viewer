//! Exact graph footprint of a transition, independent of trimmed-domain ownership.
use super::*;
impl Patch {
    /// Complete footprint is X=U, Y=(2V-1)w(U), with 0<w<1. Original
    /// contact curves are freshly lifted. The caller must still prove that
    /// the retained domain is the complementary graph notch and own its edges.
    pub fn inspect_retained_projection(&self, side: usize, b: &mut Budget) -> Result<bool> {
        check(side < 2, "Choose original source side")?;
        self.source_contact(side, b)?;
        let n = self.coefficients.len() - 1;
        let m = self.coefficients[0].len() - 1;
        let width = self
            .width
            .control_points
            .iter()
            .map(|p| b.authored(p[0]))
            .collect::<Result<Vec<_>>>()?;
        if width.iter().any(|x| x <= &Q::zero() || x >= &Q::one()) {
            return Ok(false);
        }
        let width = elevate(&width, n, b)?;
        let domain = self.width.domain();
        let a = b.authored(domain[0])?;
        let z = b.authored(domain[1])?;
        for (i, row) in self.coefficients.iter().enumerate() {
            let x = b.keep(&a + (&z - &a) * ratio(i as u64, n as u64))?;
            for (j, p) in row.iter().enumerate() {
                let factor =
                    b.keep(Q::from_integer(2.into()) * ratio(j as u64, m as u64) - Q::one())?;
                let y = b.keep(&width[i] * factor)?;
                if !b.keep(&p[0] - &x)?.is_zero() || !b.keep(&p[1] - y)?.is_zero() {
                    return Ok(false);
                }
            }
        }
        Ok(true)
    }
}
