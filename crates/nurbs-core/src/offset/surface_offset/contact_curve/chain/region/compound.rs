//! Sealed outer contact region with arbitrary certified hole/island nesting.
use super::{Curve, Region, admit_uv, lift, rails};
use crate::{Result, check, trim_region_audit::nesting};
pub mod cap;
#[derive(Clone, Copy)]
pub struct Limits {
    pub nesting: nesting::Limits,
    /// Independent full degree-refinement allowance per additional UV edge.
    pub fit: lift::FitLimits,
    /// Independent final allowance per actual outer/inner loop edge.
    pub final_lift: lift::Limits,
}
impl Default for Limits {
    fn default() -> Self {
        Self {
            nesting: nesting::Limits::default(),
            fit: lift::FitLimits::default(),
            final_lift: lift::Limits {
                cells: 100000,
                spans: 128,
            },
        }
    }
}
pub struct Compound {
    outer: Region,
    inputs: Vec<Vec<Curve>>,
    limits: Limits,
    built: Built,
}
struct Built {
    domain: nesting::Domain,
    world: Vec<Vec<Curve>>,
    fits: Vec<Vec<lift::FitReport>>,
    lifts: Vec<Vec<lift::Report>>,
}
impl Compound {
    pub fn cap(
        self,
        normal_tolerance: f64,
        boundary_tolerance: f64,
        limits: cap::Limits,
    ) -> Result<cap::Cap> {
        cap::build(self, normal_tolerance, boundary_tolerance, limits)
    }
    pub fn outer(&self) -> &Region {
        &self.outer
    }
    pub fn uv_domain(&self) -> &nesting::Domain {
        &self.built.domain
    }
    pub fn world_loops(&self) -> &[Vec<Curve>] {
        &self.built.world
    }
    pub fn inner_fitting(&self) -> &[Vec<lift::FitReport>] {
        &self.built.fits
    }
    pub fn lift_proofs(&self) -> &[Vec<lift::Report>] {
        &self.built.lifts
    }
    pub fn classify(
        &self,
        rectangle: [[f64; 2]; 2],
        max_cells: usize,
    ) -> Result<nesting::Classification> {
        self.built.domain.classify(rectangle, max_cells)
    }
    pub fn reverify(&self) -> Result<()> {
        self.outer.reverify()?;
        let r = assemble(&self.outer, &self.inputs, self.limits)?;
        check(
            r.domain.loops() == self.built.domain.loops()
                && r.domain.nodes() == self.built.domain.nodes()
                && r.world == self.built.world,
            "Nested offset region differs from full original reconstruction",
        )
    }
}
fn assemble(outer: &Region, inputs: &[Vec<Curve>], limits: Limits) -> Result<Built> {
    lift::fitting::validate(limits.fit)?;
    check(
        (1..=100000).contains(&limits.final_lift.cells)
            && (1..=100000).contains(&limits.final_lift.spans),
        "Choose bounded actual nested-region lift work",
    )?;
    check(
        (1..=64).contains(&limits.nesting.max_loops)
            && inputs.len() < limits.nesting.max_loops
            && (2..=256).contains(&limits.nesting.max_curves)
            && outer.uv.len() <= limits.nesting.max_curves
            && inputs
                .iter()
                .all(|l| (2..=limits.nesting.max_curves).contains(&l.len()))
            && outer.uv.len() + inputs.iter().map(Vec::len).sum::<usize>()
                <= limits.nesting.max_curves,
        "Bound nested-loop input counts before copying or examining coefficients",
    )?;
    let source = &outer.chain.sources[outer.side];
    let distance = outer.chain.distances[outer.side];
    // The outer is an immutable sealed Region, not a caller-positive report.
    // Its actual definitions are retained, and ALL final world lifts and the
    // complete nesting forest are independently regenerated below.
    admit_uv(inputs.iter().flatten(), source)?;
    let mut loops = vec![outer.uv.clone()];
    loops.extend_from_slice(inputs);
    let domain = nesting::build(&loops, outer.chain.tolerance_uv, limits.nesting)?;
    check(
        domain.nodes()[0].parent.is_none()
            && (1..loops.len()).all(|i| domain.proof().inside[i][0] == Some(true)),
        "Additional nested loops must lie strictly inside the original outer region",
    )?;
    let mut world = vec![outer.world.clone()];
    let mut fits = vec![];
    for uv in inputs {
        let mut vertices = vec![];
        for c in uv {
            let p = &c.control_points[0];
            vertices.push(rails::corner(
                source,
                distance,
                [p[0], p[1]],
                limits.fit.spans,
            )?);
        }
        let mut curves = vec![];
        let mut reports = vec![];
        for (i, c) in uv.iter().enumerate() {
            let report = lift::fit(
                c,
                source,
                distance,
                outer.chain.tolerance_mm,
                Some([vertices[i], vertices[(i + 1) % vertices.len()]]),
                limits.fit,
            )?;
            curves.push(
                report
                    .curve
                    .as_ref()
                    .ok_or_else(|| crate::input("Nested boundary world lift remains unqualified"))?
                    .clone(),
            );
            reports.push(report);
        }
        rails::exact_world_joins(&curves)?;
        world.push(curves);
        fits.push(reports);
    }
    let mut lifts = vec![];
    for (uv, world) in loops.iter().zip(&world) {
        let mut proofs = vec![];
        for (u, w) in uv.iter().zip(world) {
            let proof = lift::certify(
                w,
                u,
                source,
                distance,
                outer.chain.tolerance_mm,
                limits.final_lift,
            )?;
            check(
                proof.lift_correspondence_proven,
                "Actual nested world/UV boundary lacks full normal-offset lift agreement",
            )?;
            proofs.push(proof);
        }
        lifts.push(proofs);
    }
    Ok(Built {
        domain,
        world,
        fits,
        lifts,
    })
}
/// Add any finite admitted hole/island hierarchy inside one original outer UV
/// region. Whole Jordan separation/containment proves even/odd material roles.
/// Canonical orientation is expressed only as coedge order/direction metadata;
/// no authored UV weights, knots or poles are modified. Each new world loop has
/// literal endpoint closure and every final edge has a fresh direct lift proof.
///
/// This proves UV fill components, not global world embedding, volumetric
/// material sectors, cap regularity or solid/BRep sewing. The outer sealed value
/// is moved into the result; full replay also reconstructs its original proofs.
pub fn build(outer: Region, loops: &[Vec<Curve>], limits: Limits) -> Result<Compound> {
    let built = assemble(&outer, loops, limits)?;
    Ok(Compound {
        outer,
        inputs: loops.to_vec(),
        limits,
        built,
    })
}
