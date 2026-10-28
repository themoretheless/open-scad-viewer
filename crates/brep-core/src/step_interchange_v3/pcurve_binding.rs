//! Bind ordinary shared-support pcurves by continuous correspondence, not order.
use super::*;
impl DirectBuilder<'_> {
    pub(super) fn matching_pcurves(
        &self,
        associations: &Value,
        surface_id: usize,
    ) -> Result<Vec<Curve>> {
        let mut matching = Vec::new();
        for p in list(associations, "SURFACE_CURVE pcurves")? {
            let (_, pa) = call(self.entities, one_ref(p, "SURFACE_CURVE pcurve")?)?;
            if pa.len() != 3 {
                return Err(refuse("Malformed PCURVE"));
            }
            if one_ref(&pa[1], "PCURVE surface")? == surface_id {
                let mut curve_id = one_ref(&pa[2], "PCURVE reference_to_curve")?;
                if self.ap242_composition {
                    let (reference_type, reference_args) = call(self.entities, curve_id)?;
                    if reference_type != "DEFINITIONAL_REPRESENTATION" || reference_args.len() != 3
                    {
                        return Err(refuse(
                            "AP242 PCURVE reference_to_curve must be a DEFINITIONAL_REPRESENTATION",
                        ));
                    }
                    let items = list(&reference_args[1], "DEFINITIONAL_REPRESENTATION items")?;
                    if items.len() != 1 {
                        return Err(refuse(
                            "PCURVE definitional representation must contain one curve",
                        ));
                    }
                    curve_id = one_ref(&items[0], "DEFINITIONAL_REPRESENTATION curve")?;
                }
                matching.push(if self.ap242_composition {
                    curve_v5(
                        self.entities,
                        curve_id,
                        2,
                        1.,
                        self.interior_point_selectors,
                    )?
                } else {
                    curve(self.entities, curve_id, 2, 1.)?
                });
            }
        }
        if matching.is_empty() {
            return Err(refuse("No PCURVE corresponds to ADVANCED_FACE surface"));
        }
        Ok(matching)
    }
    pub(super) fn select_pcurve(
        &mut self,
        id: usize,
        surface_id: usize,
        kind: &str,
        matching: Vec<Curve>,
        world: &Curve,
        surface: &Surface,
        reversed: bool,
        degenerate: bool,
        require_exact: bool,
    ) -> Result<Curve> {
        // Periodic shore and pole association retains the existing ordered
        // contract. World correspondence alone cannot choose a lifted UV shore.
        if matching.len() == 1 && kind != "SEAM_CURVE" {
            let pc = matching.into_iter().next().unwrap();
            verify_correspondence(
                world,
                &pc,
                surface,
                reversed,
                self.whole_domain_proofs,
                require_exact,
            )?;
            return Ok(pc);
        }
        if kind == "SEAM_CURVE" || surface.periodic_u || surface.periodic_v || degenerate {
            let cursor = self.seam_use.entry((id, surface_id)).or_insert(0);
            let pc = matching
                .get(*cursor)
                .ok_or_else(|| refuse("Surface curve has fewer pcurves than oriented uses"))?
                .clone();
            verify_correspondence(
                world,
                &pc,
                surface,
                reversed,
                self.whole_domain_proofs,
                require_exact,
            )?;
            *cursor += 1;
            return Ok(pc);
        }
        choose_regular(
            world,
            surface,
            reversed,
            self.whole_domain_proofs,
            require_exact,
            &matching,
            self.pcurve_use.entry((id, surface_id)).or_default(),
        )
    }
}
fn choose_regular(
    world: &Curve,
    surface: &Surface,
    reversed: bool,
    whole: bool,
    require_exact: bool,
    matching: &[Curve],
    used: &mut BTreeSet<usize>,
) -> Result<Curve> {
    choose_regular_with_budget(
        world,
        surface,
        reversed,
        whole,
        require_exact,
        matching,
        used,
        100000,
    )
}
fn choose_regular_with_budget(
    world: &Curve,
    surface: &Surface,
    reversed: bool,
    whole: bool,
    require_exact: bool,
    matching: &[Curve],
    used: &mut BTreeSet<usize>,
    max_cells: usize,
) -> Result<Curve> {
    use nurbs_core::curve_surface_agreement::{self, Status};
    let mut candidates = Vec::new();
    let mut cells = 0;
    for (i, pc) in matching
        .iter()
        .enumerate()
        .filter(|(i, _)| !used.contains(i))
    {
        if cells == max_cells {
            return Err(refuse(
                "Shared-support PCURVE correspondence budget exhausted",
            ));
        }
        let proof =
            curve_surface_agreement::verify(world, pc, surface, reversed, 1e-5, max_cells - cells)?;
        cells += proof.cells;
        match proof.status {
            Status::Mismatch => {}
            Status::Unresolved => {
                return Err(refuse(
                    "Shared-support PCURVE continuous correspondence is unresolved",
                ));
            }
            Status::WithinTolerance => {
                verify_correspondence(world, pc, surface, reversed, whole, require_exact)?;
                candidates.push(i);
            }
        }
    }
    let Some(&selected) = candidates.first() else {
        return Err(refuse(
            "Surface curve has no unused continuously corresponding PCURVE",
        ));
    };
    if candidates
        .iter()
        .any(|&i| matching[i] != matching[selected])
    {
        return Err(refuse(
            "Ambiguous shared-support PCURVE correspondence; native clocks or UV definitions differ",
        ));
    }
    used.insert(selected);
    Ok(matching[selected].clone())
}

#[cfg(test)]
mod tests {
    use super::*;
    fn surface() -> Surface {
        Surface {
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
        }
    }
    #[test]
    fn association_order_does_not_replace_oriented_correspondence_or_reuse_a_pcurve() {
        let s = surface();
        let world = crate::line(vec![0., 0.25, 0.], vec![1., 0.25, 0.]);
        let forward = crate::line(vec![0., 0.25], vec![1., 0.25]);
        let back = forward.reverse().unwrap();
        for pcs in [
            vec![forward.clone(), back.clone()],
            vec![back.clone(), forward.clone()],
        ] {
            let mut used = BTreeSet::new();
            assert_eq!(
                choose_regular(&world, &s, false, true, false, &pcs, &mut used).unwrap(),
                forward
            );
            assert!(choose_regular(&world, &s, false, true, false, &pcs, &mut used).is_err());
            assert_eq!(used.len(), 1);
            assert_eq!(
                choose_regular(&world, &s, true, true, false, &pcs, &mut used).unwrap(),
                back
            );
            assert_eq!(used.len(), 2);
        }
    }
    #[test]
    fn sub_tolerance_or_native_clock_ambiguity_is_not_a_nearest_match() {
        let s = surface();
        let world = crate::line(vec![0., 0.25, 0.], vec![1., 0.25, 0.]);
        let pc = crate::line(vec![0., 0.25], vec![1., 0.25]);
        let mut other = pc.clone();
        for p in &mut other.control_points {
            p[1] += 1e-10;
        }
        assert!(
            choose_regular(
                &world,
                &s,
                false,
                true,
                false,
                &[pc.clone(), other],
                &mut BTreeSet::new()
            )
            .is_err()
        );
        let mut clock = pc.clone();
        clock.knots = vec![10., 10., 14., 14.];
        assert!(
            choose_regular(
                &world,
                &s,
                false,
                true,
                false,
                &[pc.clone(), clock],
                &mut BTreeSet::new()
            )
            .is_err()
        );
        assert_eq!(
            choose_regular(
                &world,
                &s,
                false,
                true,
                false,
                &[pc.clone(), pc.clone()],
                &mut BTreeSet::new()
            )
            .unwrap(),
            pc
        );
    }
    #[test]
    fn inconclusive_competitor_cannot_be_discarded_to_claim_a_unique_match() {
        let s = surface();
        let world = crate::line(vec![0., 0.25, 0.], vec![1., 0.25, 0.]);
        let pc = crate::line(vec![0., 0.25], vec![1., 0.25]);
        let hard = Curve {
            degree: 2,
            knots: vec![0., 0., 0., 1., 1., 1.],
            control_points: vec![vec![0., 0.25], vec![0.5, 0.25], vec![1., 0.25]],
            weights: vec![1., 0.875, 1.],
            periodic: false,
        };
        let proof =
            nurbs_core::curve_surface_agreement::verify(&world, &hard, &s, false, 1e-5, 1).unwrap();
        assert_eq!(
            proof.status,
            nurbs_core::curve_surface_agreement::Status::Unresolved
        );
        let mut used = BTreeSet::new();
        assert!(
            choose_regular_with_budget(&world, &s, false, true, false, &[pc, hard], &mut used, 2)
                .is_err()
        );
        assert!(used.is_empty());
    }
    #[test]
    fn one_matching_seam_pcurve_is_not_reused_after_its_only_oriented_use() {
        let entities = BTreeMap::new();
        let mut b = DirectBuilder {
            entities: &entities,
            scale: 1.,
            analytic_surfaces: true,
            ap242_composition: true,
            allow_degenerate: true,
            whole_domain_proofs: true,
            interior_point_selectors: true,
            allow_open_shells: true,
            vertices: vec![],
            vertex_map: BTreeMap::new(),
            edges: vec![],
            edge_map: BTreeMap::new(),
            seam_use: BTreeMap::new(),
            pcurve_use: BTreeMap::new(),
            loops: vec![],
            loop_entity: vec![],
            faces: vec![],
            face_entity: vec![],
            shells: vec![],
            shell_entity: vec![],
            bodies: vec![],
            body_entity: vec![],
        };
        let s = surface();
        let world = crate::line(vec![0., 0.25, 0.], vec![1., 0.25, 0.]);
        let pc = crate::line(vec![0., 0.25], vec![1., 0.25]);
        b.select_pcurve(
            1,
            2,
            "SEAM_CURVE",
            vec![pc.clone()],
            &world,
            &s,
            false,
            false,
            false,
        )
        .unwrap();
        assert!(
            b.select_pcurve(
                1,
                2,
                "SEAM_CURVE",
                vec![pc],
                &world,
                &s,
                false,
                false,
                false
            )
            .is_err()
        );
    }
}
