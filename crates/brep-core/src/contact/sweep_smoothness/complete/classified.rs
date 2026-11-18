//! Complete explicit mixed C0/G1/G2 edge coverage of an actual native model.
//! Global embedding/material roles remain independent constructor obligations.
use super::*;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Class {
    C0,
    G1,
    G2,
}
impl Class {
    pub fn order(self) -> usize {
        match self {
            Self::C0 => 0,
            Self::G1 => 1,
            Self::G2 => 2,
        }
    }
}
#[derive(Clone, Copy)]
pub struct Limits {
    pub incidence_steps: usize,
    pub exact_work: u64,
    pub agreement_work: u64,
    pub regularity_cells: usize,
}
pub struct Report {
    pub certified: bool,
    pub classes: Vec<Class>,
    pub steps: usize,
    pub smooth_edges: Vec<usize>,
    pub c0_edges: Vec<usize>,
    pub uncertain_edge: Option<usize>,
    pub agreement: Option<crate::boundary_agreement::ExactReport>,
    pub regularity: Option<crate::boundary_embedding::regularity::Report>,
    pub vertex_links_complete: bool,
    pub proof: Option<ProjectiveSeamCollectionReport>,
    pub exact_work: u64,
    pub reason: &'static str,
}
/// Bounds on the caller-supplied shared work.
const MAX_INCIDENCE_STEPS: usize = 1_000_000;
const MAX_AGREEMENT_WORK: u64 = 1_000_000;
const MAX_REGULARITY_CELLS: usize = 100_000;
fn step(out: &mut Report, max: usize) -> bool {
    if out.steps == max {
        out.reason = "classified-boundary-incidence-work-limit";
        false
    } else {
        out.steps += 1;
        true
    }
}
/// One owned edge use: face, full parameter side and increasing direction.
type EdgeUse = (usize, Option<(&'static str, bool)>, bool);
/// Smooth seams awaiting the shared exact jet proof.
struct SmoothSeams {
    patches: Vec<Surface>,
    seams: Vec<ProjectiveSeam>,
}
/// Bounded classified-boundary inspection; every stage returning `None`
/// has already recorded its refusal reason in `out`.
struct Inspection<'a> {
    model: &'a Model,
    classes: &'a [Class],
    limits: Limits,
    out: Report,
}
impl Inspection<'_> {
    fn collect_uses(&mut self) -> Result<Option<Vec<Vec<EdgeUse>>>> {
        let model = self.model;
        let mut uses = vec![Vec::new(); model.edges.len()];
        for (face, f) in model.faces.iter().enumerate() {
            if !step(&mut self.out, self.limits.incidence_steps) {
                return Ok(None);
            }
            for wire in std::iter::once(f.outer).chain(f.holes.iter().copied()) {
                let wire = model
                    .loops
                    .get(wire)
                    .ok_or_else(|| crate::invalid("Invalid classified boundary loop"))?;
                for c in &wire.coedges {
                    if !step(&mut self.out, self.limits.incidence_steps) {
                        return Ok(None);
                    }
                    let side = periodic_uses::full_use(model, c)
                        .then(|| coedge_side(&f.surface, &c.pcurve, c.reversed))
                        .flatten();
                    let owned = uses
                        .get_mut(c.edge)
                        .ok_or_else(|| crate::invalid("Invalid classified boundary edge"))?;
                    owned.push((face, side, c.reversed));
                }
            }
        }
        Ok(Some(uses))
    }
    fn smooth_seams(&mut self, uses: &[Vec<EdgeUse>]) -> Result<Option<SmoothSeams>> {
        let mut smooth = SmoothSeams {
            patches: Vec::new(),
            seams: Vec::new(),
        };
        for ((edge, owned), &class) in uses.iter().enumerate().zip(self.classes) {
            self.out.uncertain_edge = Some(edge);
            if !step(&mut self.out, self.limits.incidence_steps) {
                return Ok(None);
            }
            if owned.len() != 2 {
                self.out.reason = "classified-boundary-two-uses-unproven";
                return Ok(None);
            }
            if class == Class::C0 {
                self.out.c0_edges.push(edge);
                continue;
            }
            let [(a, Some((ab, ad)), ar), (b, Some((bb, bd)), br)] = owned.as_slice() else {
                self.out.reason = "classified-boundary-natural-carrier-unproven";
                return Ok(None);
            };
            let carrier = &self.model.faces[*a].surface;
            if a == b && periodic_uses::opposite_sides(carrier, ab, bb) {
                if ad != bd || ar == br {
                    self.out.reason = "classified-boundary-wrap-orientation-unproven";
                    return Ok(None);
                }
                let (opened, used) =
                    periodic_uses::open(carrier, self.limits.exact_work - self.out.exact_work)?;
                self.out.exact_work += used;
                let Some(opened) = opened else {
                    self.out.reason = "classified-boundary-periodic-opening-unproven";
                    return Ok(None);
                };
                let patch = smooth.patches.len();
                smooth.patches.push(opened.into_owned());
                smooth.seams.push(ProjectiveSeam {
                    patches: [patch, patch],
                    boundaries: [(*ab).into(), (*bb).into()],
                    order: class.order(),
                    normal_scale: 1.,
                });
                self.out.smooth_edges.push(edge);
                continue;
            }
            let left = carrier.clone();
            let mut right = self.model.faces[*b].surface.clone();
            if ad != bd {
                let budget = self.limits.exact_work - self.out.exact_work;
                let (accepted, used) = reverse_along(&mut right, bb, budget)?;
                self.out.exact_work += used;
                if !accepted {
                    self.out.reason = "classified-boundary-clock-unproven";
                    return Ok(None);
                }
            }
            let start = smooth.patches.len();
            smooth.patches.push(left);
            smooth.patches.push(right);
            smooth.seams.push(ProjectiveSeam {
                patches: [start, start + 1],
                boundaries: [(*ab).into(), (*bb).into()],
                order: class.order(),
                normal_scale: 1.,
            });
            self.out.smooth_edges.push(edge);
        }
        self.out.uncertain_edge = None;
        Ok(Some(smooth))
    }
    /// World/UV agreement, vertex links and face regularity, in that order.
    fn c0_obligations(&mut self) -> Result<bool> {
        if self.limits.agreement_work == 0 {
            self.out.reason = "classified-boundary-agreement-work-limit";
            return Ok(false);
        }
        let agreement =
            crate::boundary_agreement::verify_exact(self.model, self.limits.agreement_work)?;
        let equal = agreement.all_equal && agreement.all_joins_exact;
        self.out.agreement = Some(agreement);
        if !equal {
            self.out.reason = "classified-boundary-position-unproven";
            return Ok(false);
        }
        self.out.vertex_links_complete = vertex_links(self.model);
        if !self.out.vertex_links_complete {
            self.out.reason = "classified-boundary-vertex-links-unproven";
            return Ok(false);
        }
        let cells = self.limits.regularity_cells;
        let regularity = crate::boundary_embedding::regularity::inspect(self.model, cells)?;
        let regular = regularity.all_faces_regular;
        self.out.regularity = Some(regularity);
        if !regular {
            self.out.reason = "classified-boundary-face-regularity-unproven";
            return Ok(false);
        }
        Ok(true)
    }
    fn exact_jets(&mut self, smooth: &SmoothSeams) -> Result<bool> {
        if smooth.seams.is_empty() {
            return Ok(true);
        }
        let proof = graph_jets::qualify(
            &smooth.patches,
            &smooth.seams,
            self.limits.exact_work - self.out.exact_work,
        )?;
        self.out.exact_work += proof.exact_work;
        let proven = proof.certified;
        self.out.proof = Some(proof);
        if !proven {
            self.out.reason = "classified-boundary-exact-jets-unproven";
        }
        Ok(proven)
    }
    fn run(&mut self) -> Result<()> {
        let Some(uses) = self.collect_uses()? else {
            return Ok(());
        };
        let Some(smooth) = self.smooth_seams(&uses)? else {
            return Ok(());
        };
        if self.c0_obligations()? && self.exact_jets(&smooth)? {
            self.out.certified = true;
            self.out.reason = "complete-explicit-actual-boundary-classes-qualified";
        }
        Ok(())
    }
}
/// Every actual edge requires a class and exactly two owned uses. C0 still
/// needs exact world/UV agreement, closed native vertex links and regular face
/// interiors. Smooth edges require fresh exact original rational tensor jets
/// at THEIR requested order. No failed G2 request is downgraded to G1 or C0.
/// One coefficient ledger covers every smooth edge, including closed seams.
pub fn inspect(model: &Model, classes: &[Class], limits: Limits) -> Result<Report> {
    model.validate_boundary_diagnostic_inputs()?;
    require(
        classes.len() == model.edges.len()
            && !classes.is_empty()
            && limits.incidence_steps <= MAX_INCIDENCE_STEPS
            && limits.exact_work <= continuity::MAX_PROJECTIVE_SEAM_COLLECTION_WORK
            && limits.agreement_work <= MAX_AGREEMENT_WORK
            && limits.regularity_cells <= MAX_REGULARITY_CELLS,
        "Classify every actual edge with bounded shared work",
    )?;
    let mut inspection = Inspection {
        model,
        classes,
        limits,
        out: Report {
            certified: false,
            classes: classes.to_vec(),
            steps: 0,
            smooth_edges: Vec::new(),
            c0_edges: Vec::new(),
            uncertain_edge: None,
            agreement: None,
            regularity: None,
            vertex_links_complete: false,
            proof: None,
            exact_work: 0,
            reason: "classified-boundary-unproven",
        },
    };
    inspection.run()?;
    Ok(inspection.out)
}

#[cfg(test)]
mod tests;
