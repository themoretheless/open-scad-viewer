//! Original rational SS equations as exact polynomial ideals, including
//! rank-deficient/tangent/pole strata. This is equation authority, not branch
//! completeness, real-root isolation, physical embedding or a Boolean result.
use super::{FiberSources, Result, Surface, check};
use num_rational::BigRational as Q;
use num_traits::{One, Zero};
mod polynomial;
mod span;
pub use polynomial::ExactSsPolynomial;
#[derive(Clone, Copy, Debug)]
pub struct ExactSsAlgebraLimits {
    pub max_work: usize,
    pub max_terms: usize,
    pub max_coefficient_bits: u64,
}
impl Default for ExactSsAlgebraLimits {
    fn default() -> Self {
        Self {
            max_work: 1000000,
            max_terms: 65536,
            max_coefficient_bits: 32768,
        }
    }
}
struct Work {
    used: usize,
    limits: ExactSsAlgebraLimits,
}
impl Work {
    fn charge(&mut self) -> Result<()> {
        if self.used == self.limits.max_work {
            return Err(crate::resource("Exact original SS algebra work exhausted"));
        }
        self.used += 1;
        Ok(())
    }
    fn coefficient(&self, c: &Q) -> Result<()> {
        if c.numer().bits() > self.limits.max_coefficient_bits
            || c.denom().bits() > self.limits.max_coefficient_bits
        {
            return Err(crate::resource(
                "Exact original SS coefficient size exhausted",
            ));
        }
        Ok(())
    }
}
fn q(x: f64) -> Result<Q> {
    Q::from_float(x).ok_or_else(|| crate::input("Nonfinite exact original SS input"))
}
fn validate_limits(l: ExactSsAlgebraLimits) -> Result<()> {
    check(
        (1..=10000000).contains(&l.max_work)
            && (1..=1000000).contains(&l.max_terms)
            && (64..=131072).contains(&l.max_coefficient_bits),
        "Invalid exact original SS algebra allowance",
    )
}
#[derive(Clone, Debug)]
pub struct QualifiedOriginalSsIdeal {
    source: FiberSources,
    spans: [usize; 4],
    homogeneous: [[ExactSsPolynomial; 4]; 2],
    equations: [ExactSsPolynomial; 3],
    limits: ExactSsAlgebraLimits,
    work: usize,
}
impl QualifiedOriginalSsIdeal {
    pub fn source(&self) -> &FiberSources {
        &self.source
    }
    pub fn spans(&self) -> [usize; 4] {
        self.spans
    }
    pub fn equations(&self) -> &[ExactSsPolynomial; 3] {
        &self.equations
    }
    pub fn homogeneous(&self) -> &[[ExactSsPolynomial; 4]; 2] {
        &self.homogeneous
    }
    pub fn work(&self) -> usize {
        self.work
    }
    pub fn reverify(&self) -> Result<()> {
        let fresh = qualify_original_ss_ideal(&self.source, self.spans, self.limits)?;
        check(
            fresh.homogeneous == self.homogeneous
                && fresh.equations == self.equations
                && fresh.work == self.work,
            "Original rational SS span equations changed",
        )
    }
}
/// Original positive weights and the original nonempty Cox spans give positive
/// denominators on each closed span patch. Thus all three cleared numerators
/// vanish iff the original surfaces agree. Source-wide C0 and exact periodic
/// seam proofs bind the span closures to the actual original evaluator.
pub fn qualify_original_ss_ideal(
    source: &FiberSources,
    spans: [usize; 4],
    limits: ExactSsAlgebraLimits,
) -> Result<QualifiedOriginalSsIdeal> {
    validate_limits(limits)?;
    source.verify_continuous_ss_source()?;
    let mut w = Work { used: 0, limits };
    let a = span::homogeneous(source.first(), [spans[0], spans[1]], 0, &mut w)?;
    let b = span::homogeneous(source.second(), [spans[2], spans[3]], 1, &mut w)?;
    let mut equations = Vec::new();
    for k in 0..3 {
        let left = a[k].product(&b[3], &mut w)?;
        let right = a[3].product(&b[k], &mut w)?;
        equations.push(left.subtract(&right, &mut w)?);
    }
    Ok(QualifiedOriginalSsIdeal {
        source: source.clone(),
        spans,
        homogeneous: [a, b],
        equations: equations.try_into().unwrap(),
        limits,
        work: w.used,
    })
}
mod groebner;
pub use groebner::{ExactSsGroebnerLimits, QualifiedSsGroebnerBasis, qualify_ss_groebner_basis};
mod division;
mod gcd;
mod deflate;
pub use deflate::{ExactSsDeflationProof,QualifiedSsDeflatedSystem,qualify_ss_deflated_system};
mod pole;
pub use pole::{QualifiedSsPoleStratum,QualifiedSsPoleSaturation,qualify_ss_pole_stratum,qualify_ss_pole_saturation};
mod fiber;
pub use fiber::{QualifiedPoleSsRootArc,qualify_pole_ss_root_arc,DeflatedSsFiberLimits,DeflatedSsFiberReport,QualifiedDeflatedSsFiber,DeflatedSsFiberResult,qualify_deflated_ss_fiber};
mod localization;
mod blowup;
pub use blowup::{QualifiedSsRootNode,QualifiedSsBlowupChart,QualifiedSsBlowupRootArc,qualify_ss_root_node,qualify_ss_blowup_chart,qualify_ss_blowup_root_arc};
mod rank_strata;
pub use rank_strata::{SsRankStrataLimits,ExactSsRankLocation,QualifiedSsRankStrata,qualify_ss_rank_strata};
mod flux;
pub(crate) use flux::bound as original_polynomial_flux;

mod clock_consequence;
pub(crate) use clock_consequence::prove as prove_clock_consequence;
pub(crate) use clock_consequence::project as project_original_ss_clock;

mod third_surface;
pub use third_surface::{QualifiedSsThirdSurfaceCandidates,qualify_ss_third_surface_candidates};
