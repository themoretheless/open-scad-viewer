//! Complete connected-component census of the original parameter root set.
//! Physical world branches, periodic quotients and branch topology differ.
use super::*;
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FiberBridgeSpec {
    pub charts: [usize; 2],
    pub domain: Box4,
}
#[derive(Clone, Debug, PartialEq)]
pub struct FiberSeparation {
    pub charts: [usize; 2],
    pub axis: usize,
    pub intervals: [[f64; 2]; 2],
}
#[derive(Clone, Debug)]
pub struct QualifiedFiberComponents {
    coverage: QualifiedMixedFiberCoverage,
    specs: Vec<FiberBridgeSpec>,
    transitions: Vec<QualifiedFiberTransition>,
    components: Vec<Vec<usize>>,
    separations: Vec<FiberSeparation>,
    rectangles: usize,
    max_rectangles: usize,
}
impl QualifiedFiberComponents {
    pub fn coverage(&self) -> &QualifiedMixedFiberCoverage {
        &self.coverage
    }
    pub fn transitions(&self) -> &[QualifiedFiberTransition] {
        &self.transitions
    }
    pub fn components(&self) -> &[Vec<usize>] {
        &self.components
    }
    pub fn separations(&self) -> &[FiberSeparation] {
        &self.separations
    }
    pub fn rectangles(&self) -> usize {
        self.rectangles
    }
    pub fn reverify(&self) -> Result<()> {
        let fresh = connect_mixed_fiber_cover(&self.coverage, &self.specs, self.max_rectangles)?;
        let fresh = fresh
            .components
            .ok_or_else(|| crate::input("Original parameter component census remains unproved"))?;
        check(
            self.components == fresh.components
                && self.separations == fresh.separations
                && self.rectangles == fresh.rectangles
                && self.transitions.len() == fresh.transitions.len()
                && self
                    .transitions
                    .iter()
                    .zip(&fresh.transitions)
                    .all(|(a, b)| a.matches(b)),
            "Original component/transition/separation census or shared work changed",
        )
    }
}
pub struct FiberComponentsResult {
    components: Option<QualifiedFiberComponents>,
    reason: &'static str,
    rectangles: usize,
}
impl FiberComponentsResult {
    pub fn components(&self) -> Option<&QualifiedFiberComponents> {
        self.components.as_ref()
    }
    pub fn into_components(self) -> Option<QualifiedFiberComponents> {
        self.components
    }
    pub fn reason(&self) -> &'static str {
        self.reason
    }
    pub fn rectangles(&self) -> usize {
        self.rectangles
    }
}
fn root_box(f: &QualifiedFiber) -> Option<Box4> {
    let image = f.report().root_image?;
    let mut box4 = f.domain();
    let mut j = 0;
    for a in 0..4 {
        if a != f.free_axis().index() {
            box4[a] = image[j];
            j += 1;
        }
    }
    Some(box4)
}
/// Prove connected components of the complete original PARAMETER root set.
/// Every chart root graph is compact and connected by original uniform
/// contraction on its closed interval. Source-owned bridges prove connections.
/// ALL chart pairs in DIFFERENT graph components must have strictly disjoint
/// original root enclosures; no omitted pair is inferred from graph structure.
/// Complete root coverage then makes these finite disjoint connected unions
/// exactly the parameter connected components of the requested product domain.
/// No physical embedding, periodic quotient, branch degree or cycle claim.
/// Coverage replay and every bridge share one original rectangle allowance.
pub fn connect_mixed_fiber_cover(
    coverage: &QualifiedMixedFiberCoverage,
    specs: &[FiberBridgeSpec],
    max_rectangles: usize,
) -> Result<FiberComponentsResult> {
    check(
        (1..=100000).contains(&max_rectangles) && specs.len() <= 256,
        "Invalid component proof work/bridge budget",
    )?;
    let fs = coverage.fibers();
    check(
        specs.iter().all(|s| {
            s.charts[0] < fs.len() && s.charts[1] < fs.len() && s.charts[0] != s.charts[1]
        }),
        "Component bridge references a missing or identical original chart",
    )?;
    let mut out = FiberComponentsResult {
        components: None,
        reason: "original-coverage-replay-unproven",
        rectangles: 0,
    };
    let mut work = Work {
        used: 0,
        max: max_rectangles,
    };
    if coverage.replay(&mut work).is_err() {
        out.rectangles = work.used;
        return Ok(out);
    }
    let mut transitions = Vec::with_capacity(specs.len());
    let mut labels = (0..fs.len()).collect::<Vec<_>>();
    for spec in specs {
        if work.used == work.max {
            out.reason = "bridge-work-unproven";
            out.rectangles = work.used;
            return Ok(out);
        }
        let r = qualify_fiber_transition(
            &fs[spec.charts[0]],
            &fs[spec.charts[1]],
            spec.domain,
            work.max - work.used,
        )?;
        work.used += r.rectangles();
        let Some(proof) = r.into_transition() else {
            out.reason = "original-bridge-unproven";
            out.rectangles = work.used;
            return Ok(out);
        };
        let a = labels[spec.charts[0]];
        let b = labels[spec.charts[1]];
        let low = a.min(b);
        let high = a.max(b);
        for label in &mut labels {
            if *label == high {
                *label = low;
            }
        }
        transitions.push(proof);
    }
    out.rectangles = work.used;
    out.reason = "cross-component-root-contact-unproven";
    let mut separations = Vec::new();
    for i in 0..fs.len() {
        for j in i + 1..fs.len() {
            if labels[i] != labels[j] {
                let (Some(a), Some(b)) = (root_box(&fs[i]), root_box(&fs[j])) else {
                    return Ok(out);
                };
                let Some(axis) = (0..4).find(|&k| a[k][1] < b[k][0] || b[k][1] < a[k][0]) else {
                    return Ok(out);
                };
                separations.push(FiberSeparation {
                    charts: [i, j],
                    axis,
                    intervals: [a[axis], b[axis]],
                });
            }
        }
    }
    let components = (0..fs.len())
        .filter(|&i| labels[i] == i)
        .map(|i| (0..fs.len()).filter(|&j| labels[j] == i).collect())
        .collect();
    out.reason = "complete-original-parameter-components";
    out.components = Some(QualifiedFiberComponents {
        coverage: coverage.clone(),
        specs: specs.to_vec(),
        transitions,
        components,
        separations,
        rectangles: work.used,
        max_rectangles,
    });
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn source_component_and_separation_receipts_cannot_be_relabelled() {
        let a = Surface {
            degree_u: 1,
            degree_v: 2,
            knots_u: vec![0., 0., 1., 1.],
            knots_v: vec![0., 0., 0., 1., 1., 1.],
            control_points: [0.25, 0.5]
                .into_iter()
                .map(|x| {
                    (0..3)
                        .map(|j| vec![x, j as f64 / 2., [0.1875, -0.3125, 0.1875][j]])
                        .collect()
                })
                .collect(),
            weights: vec![vec![1.; 3]; 2],
            periodic_u: false,
            periodic_v: false,
        };
        let b = Surface {
            degree_u: 1,
            degree_v: 1,
            knots_u: vec![0., 0., 1., 1.],
            knots_v: vec![0., 0., 1., 1.],
            control_points: vec![
                vec![vec![0., 0., 0.], vec![0., 1., 0.]],
                vec![vec![1., 0., 0.], vec![1., 1., 0.]],
            ],
            weights: vec![vec![1.; 2]; 2],
            periodic_u: false,
            periodic_v: false,
        };
        let source = FiberSources::new(&a, &b).unwrap();
        let fs = [([0.2, 0.3], [0.1, 0.4]), ([0.7, 0.8], [0.6, 0.9])]
            .into_iter()
            .map(|(v, t)| {
                source
                    .certify(
                        [[0., 1.], v, [0.2, 0.55], t],
                        FiberAxis::FirstU,
                        FiberLimits::default(),
                    )
                    .unwrap()
                    .into_fiber()
                    .unwrap()
            })
            .collect::<Vec<_>>();
        let cover = cover_mixed_fiber_surfaces(&fs, CoverageLimits::default())
            .unwrap()
            .into_coverage()
            .unwrap();
        let owner = connect_mixed_fiber_cover(&cover, &[], 100000)
            .unwrap()
            .into_components()
            .unwrap();
        owner.reverify().unwrap();
        let mut bad = owner.clone();
        bad.components = vec![vec![0, 1]];
        assert!(bad.reverify().is_err());
        let mut bad = owner.clone();
        bad.components[0].clear();
        assert!(bad.reverify().is_err());
        let mut bad = owner.clone();
        bad.separations.clear();
        assert!(bad.reverify().is_err());
        let mut bad = owner.clone();
        bad.separations[0].intervals[0][0] -= 0.01;
        assert!(bad.reverify().is_err());
        let mut bad = owner.clone();
        bad.separations[0].charts = [0, 0];
        assert!(bad.reverify().is_err());
        let mut bad = owner;
        bad.rectangles -= 1;
        assert!(bad.reverify().is_err());
    }
}
