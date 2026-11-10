use super::*;
pub struct ReplayLimits {
    /// Audit limits apply independently to every replay step.
    pub region: Limits,
    pub search_cells: usize,
    pub point_checks: usize,
    pub mapping_cells: usize,
    pub driver_cells: usize,
    pub membership_cells: usize,
    pub controls: usize,
    pub winding_cells: usize,
    pub steps: usize,
}
fn error(message: &str) -> Error {
    Error::new("BREP_SOURCE_REGION_RESTORE", message)
}
impl SourceRegion {
    /// Fresh native replay of the original equations, selectors and region gates.
    /// The same bounded per-step limits as wire restoration apply; cached loops,
    /// winding, root intervals and whole-chart flags are never used as authority.
    pub fn requalify(&self, limits: &ReplayLimits) -> Result<Self> {
        if !(1..=64).contains(&limits.steps) {
            return Err(error("Bound region replay depth to 1..64"));
        }
        replay(&self.recipe, limits, limits.steps)
    }
}
fn replay(recipe: &Recipe, l: &ReplayLimits, remaining: usize) -> Result<SourceRegion> {
    if remaining == 0 {
        return Err(error("Region replay depth or version refused"));
    }
    let original = match recipe {
        Recipe::SplitParameter {
            parent,
            address: [wire, edge],
            parameter,
        } => {
            return replay(parent, l, remaining - 1)?
                .split_boundary_parameter(*wire, *edge, *parameter);
        }
        Recipe::SplitRoot {
            parent,
            address: [wire, edge],
            point,
            role,
        } => {
            let parent = replay(parent, l, remaining - 1)?;
            let report = crate::source_contact_point::qualify(
                point.surface(),
                point.boundary(),
                point.contact(),
                point.selector(),
                l.mapping_cells,
            )?;
            let fresh = report
                .point
                .ok_or_else(|| error("Source split root is unqualified"))?;
            return parent.split_boundary(*wire, *edge, &fresh, *role);
        }
        Recipe::Original(o) | Recipe::Cut { original: o, .. } => o,
    };
    if original.wires.is_empty()
        || original.wires.len() > 16
        || original.wires.iter().any(|w| w.is_empty() || w.len() > 256)
    {
        return Err(error("Bound original wires and edges"));
    }
    let context = ToleranceContext::new(original.context.clone())
        .map_err(|_| error("Invalid original tolerance context"))?;
    let o = original;
    match recipe {
        Recipe::Original(_) => {
            let r = qualify_original_region(&context, &o.surface, &o.wires, o.tolerance, l.region)?;
            r.region.ok_or_else(|| error(r.reason))
        }
        Recipe::Cut {
            kind,
            contact,
            address: [wire, start, end],
            target_width,
            axis,
            ..
        } => match kind {
            CutKind::Linear => {
                let r = qualify_linear_region(
                    &context,
                    &o.surface,
                    &o.wires,
                    contact,
                    *wire,
                    *start,
                    *end,
                    o.tolerance,
                    l.region,
                    l.search_cells,
                    *target_width,
                    l.point_checks,
                    l.mapping_cells,
                    *axis,
                    l.driver_cells,
                    l.membership_cells,
                    l.controls,
                )?;
                r.region.ok_or_else(|| error(r.reason))
            }
            CutKind::Curved => {
                let r = qualify_curved_region(
                    &context,
                    &o.surface,
                    &o.wires,
                    contact,
                    *wire,
                    *start,
                    *end,
                    o.tolerance,
                    l.region,
                    l.search_cells,
                    *target_width,
                    l.point_checks,
                    l.mapping_cells,
                    *axis,
                    l.driver_cells,
                    l.membership_cells,
                    l.winding_cells,
                )?;
                r.region.ok_or_else(|| error(r.reason))
            }
        },
        _ => unreachable!("Split recipes returned before original-region replay"),
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    fn fixture() -> (SourceRegion, ReplayLimits) {
        let surface = Surface {
            degree_u: 1,
            degree_v: 1,
            knots_u: vec![0., 0., 1., 1.],
            knots_v: vec![0., 0., 1., 1.],
            control_points: vec![
                vec![vec![0., 0., 1.], vec![0., 1., 1.]],
                vec![vec![1., 0., 1.], vec![1., 1., 1.]],
            ],
            weights: vec![vec![1.; 2]; 2],
            periodic_u: false,
            periodic_v: false,
        };
        let corners = [vec![0., 0.], vec![1., 0.], vec![1., 1.], vec![0., 1.]];
        let wire = (0..4)
            .map(|i| {
                let pcurve =
                    Curve::from_polyline(vec![corners[i].clone(), corners[(i + 1) % 4].clone()])
                        .unwrap();
                let mut curve = pcurve.clone();
                for p in &mut curve.control_points {
                    p.push(1.);
                }
                Boundary {
                    curve,
                    pcurve,
                    reversed: false,
                }
            })
            .collect::<Vec<_>>();
        let region_limits = Limits {
            pairs: 10000,
            region_cells: 100000,
            domain_cells: 100000,
            agreement_cells: 100000,
        };
        let region = qualify_original_region(
            &ToleranceContext::default_valid(),
            &surface,
            &[wire],
            1e-8,
            region_limits,
        )
        .unwrap()
        .region
        .unwrap();
        let limits = ReplayLimits {
            region: region_limits,
            search_cells: 10000,
            point_checks: 10000,
            mapping_cells: 10000,
            driver_cells: 10000,
            membership_cells: 10000,
            controls: 10000,
            winding_cells: 10000,
            steps: 64,
        };
        (region, limits)
    }
    #[test]
    fn native_root_split_replays_original_crossing_and_mapping_budget() {
        let (original, mut limits) = fixture();
        let edge = &original.loops()[0][0];
        let contact = Curve::from_polyline(vec![vec![0.5, -0.25], vec![0.5, 0.25]]).unwrap();
        let point = crate::source_contact_point::qualify(
            edge.surface(),
            edge.curve(),
            &contact,
            [[0., 1.]; 2],
            10000,
        )
        .unwrap()
        .point
        .unwrap();
        let split = original
            .split_boundary(0, 0, &point, Role::Boundary)
            .unwrap();
        let fresh = split.requalify(&limits).unwrap();
        assert_eq!(fresh.loops()[0].len(), 5);
        assert!(
            fresh.loops()[0]
                .iter()
                .zip(&split.loops()[0])
                .all(|(a, b)| a.same_definition(b))
        );
        assert!(fresh.loops()[0][0].joins(&fresh.loops()[0][1]));
        limits.steps = 1;
        assert!(split.requalify(&limits).is_err());
        limits.steps = 2;
        limits.mapping_cells = 0;
        assert!(split.requalify(&limits).is_err());
        #[cfg(feature = "codec")]
        crate::source_region_restore::assert_replay(&split);
    }
    #[test]
    fn native_region_replay_recomputes_cached_geometry_and_enforces_depth() {
        let (mut original, mut limits) = fixture();
        let expected = original.loops().to_vec();
        original.loops.clear();
        original.whole_chart = false;
        original.chart_winding = -100;
        original.source_loop_indices = vec![99];
        let fresh = original.requalify(&limits).unwrap();
        assert!(fresh.whole_chart_material());
        assert_eq!(fresh.chart_winding(), 1);
        assert_eq!(fresh.source_loop_indices(), [0]);
        assert_eq!(fresh.loops().len(), expected.len());
        assert!(
            fresh.loops()[0]
                .iter()
                .zip(&expected[0])
                .all(|(a, b)| a.same_definition(b))
        );
        let split = fresh.split_boundary_parameter(0, 0, 0.5).unwrap();
        limits.steps = 1;
        assert!(split.requalify(&limits).is_err());
        limits.steps = 2;
        assert_eq!(split.requalify(&limits).unwrap().loops()[0].len(), 5);
        limits.steps = 0;
        assert!(fresh.requalify(&limits).is_err());
        limits.steps = 65;
        assert!(fresh.requalify(&limits).is_err());
    }
    #[test]
    fn native_region_replay_refuses_changed_original_equations_and_exhausted_work() {
        let (mut original, mut limits) = fixture();
        limits.region.agreement_cells = 0;
        assert!(original.requalify(&limits).is_err());
        limits.region.agreement_cells = 100000;
        let Recipe::Original(ref mut inputs) = original.recipe else {
            unreachable!()
        };
        inputs.wires[0][0].curve.control_points[0][2] = 2.;
        assert!(original.requalify(&limits).is_err());
    }
}
