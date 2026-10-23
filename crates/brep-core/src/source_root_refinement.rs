//! Fresh nested selectors for the same privately qualified original UV root.
//! Enclosures only: no Cartesian Model vertex, topology weld or STEP admission.
use crate::source_contact_point::{self, SourcePoint};
use nurbs_core::{Error, Result, interval_eval::Interval as I};
pub struct Refinement {
    original: SourcePoint,
    refined: SourcePoint,
    diameter_bound: [f64; 2],
}
impl Refinement {
    pub fn original(&self) -> &SourcePoint {
        &self.original
    }
    pub fn refined(&self) -> &SourcePoint {
        &self.refined
    }
    pub fn diameter_bound(&self) -> [f64; 2] {
        self.diameter_bound
    }
}
pub struct Report {
    pub refinement: Option<Refinement>,
    pub root_checks: usize,
    pub mapping_cells: usize,
    pub reason: &'static str,
}
pub fn qualify(
    original: &SourcePoint,
    tolerance_mm: f64,
    max_checks: usize,
    max_mapping: usize,
) -> Result<Report> {
    if !tolerance_mm.is_finite()
        || tolerance_mm <= 0.
        || !(1..=128).contains(&max_checks)
        || !(1..=100000).contains(&max_mapping)
    {
        return Err(Error::new(
            "BREP_SOURCE_ROOT_REFINEMENT",
            "Choose positive tolerance and bounded root work",
        ));
    }
    let mut out = Report {
        refinement: None,
        root_checks: 0,
        mapping_cells: 0,
        reason: "source-root-refinement-work-limit",
    };
    let mut current = original.clone();
    loop {
        let mut diameter = I::point(0.);
        for bounds in current.world_box() {
            let width = I::point(bounds[1]).sub(I::point(bounds[0]))?;
            diameter = diameter.add(width)?;
        }
        if diameter.hi <= tolerance_mm {
            out.refinement = Some(Refinement {
                original: original.clone(),
                refined: current,
                diameter_bound: [0., diameter.hi],
            });
            out.reason = "source-root-refinement-qualified";
            return Ok(out);
        }
        if out.root_checks == max_checks || out.mapping_cells == max_mapping {
            return Ok(out);
        }
        let selector = std::array::from_fn(|axis| {
            let old = original.selector()[axis];
            let refined = current.root_parameters()[axis];
            [old[0].max(refined[0]), old[1].min(refined[1])]
        });
        if selector.iter().any(|r| r[0] >= r[1]) {
            out.reason = "source-root-refinement-selector-unproven";
            return Ok(out);
        }
        let replay = source_contact_point::qualify(
            current.surface(),
            current.boundary(),
            current.contact(),
            selector,
            max_mapping - out.mapping_cells,
        )?;
        out.root_checks += 1;
        out.mapping_cells += replay.mapping_cells;
        let Some(refined) = replay.point else {
            out.reason = "source-root-refinement-replay-unproven";
            return Ok(out);
        };
        // Both roots exist in a nested selector of the original Unique box.
        // Its fresh uniqueness is identity authority, never interval overlap alone.
        if refined.root_parameters() == current.root_parameters()
            && refined.world_box() == current.world_box()
        {
            out.reason = "source-root-refinement-stagnant";
            return Ok(out);
        }
        current = refined;
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    use nurbs_core::{curve::Curve, surface::Surface};
    #[test]
    fn irrational_root_refines_without_replacing_original_recipe() {
        let surface = Surface {
            degree_u: 1,
            degree_v: 1,
            periodic_u: false,
            periodic_v: false,
            knots_u: vec![0., 0., 1., 1.],
            knots_v: vec![-1., -1., 1., 1.],
            control_points: vec![
                vec![vec![0., -1., 0.], vec![0., 1., 0.]],
                vec![vec![1., -1., 0.], vec![1., 1., 0.]],
            ],
            weights: vec![vec![1.; 2]; 2],
        };
        let main = Curve::from_polyline(vec![vec![0., 0.], vec![1., 0.]]).unwrap();
        let cutter = Curve {
            degree: 2,
            periodic: false,
            knots: vec![0., 0., 0., 1., 1., 1.],
            control_points: vec![vec![0., -0.5], vec![0.5, -0.5], vec![1., 0.5]],
            weights: vec![1.; 3],
        };
        let point = source_contact_point::qualify(&surface, &main, &cutter, [[0.7, 0.71]; 2], 100)
            .unwrap()
            .point
            .unwrap();
        let report = qualify(&point, 1e-8, 16, 1000).unwrap();
        let proof = report.refinement.expect(report.reason);
        assert_eq!(proof.original().definition(), point.definition());
        assert_eq!(proof.refined().boundary(), point.boundary());
        assert_eq!(proof.refined().contact(), point.contact());
        assert!(proof.diameter_bound()[1] <= 1e-8);
        for axis in 0..2 {
            let bound = proof.refined().root_parameters()[axis];
            assert!(
                bound[0] <= std::f64::consts::FRAC_1_SQRT_2
                    && bound[1] >= std::f64::consts::FRAC_1_SQRT_2
            );
            assert!(proof.refined().selector()[axis][0] >= point.selector()[axis][0]);
            assert!(proof.refined().selector()[axis][1] <= point.selector()[axis][1]);
        }
        assert!(report.root_checks > 0 && report.root_checks <= 16 && report.mapping_cells <= 1000);
        assert!(
            qualify(&point, 1e-100, 1, 1000)
                .unwrap()
                .refinement
                .is_none()
        );
        assert!(qualify(&point, 0., 16, 1000).is_err());
    }
}
