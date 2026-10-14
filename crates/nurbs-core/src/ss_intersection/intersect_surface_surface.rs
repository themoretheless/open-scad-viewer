use super::*;
pub fn intersect_surface_surface_report(
    first: &Surface,
    second: &Surface,
    tolerance: Option<ToleranceContext>,
) -> Result<SurfaceSurfaceIntersection> {
    admit_surface(first)?;
    admit_surface(second)?;
    let tolerance = context(tolerance);
    let floor = tolerance.parametric_bounds().floor.max(1e-12);
    let dist_floor = tolerance.spatial_bounds().absolute_mm.max(1e-9);
    let mut components = Vec::new();
    let mut unresolved = Vec::new();
    let mut boxes_visited = 0_usize;
    let mut bernstein_excluded = 0_usize;
    let mut krawczyk_isolated = 0_usize;
    let mut used_seeds: Vec<([f64; 2], [f64; 2])> = Vec::new();

    // Exact planar reduction when both charts are affine planes.
    if let Some(planar) = plane_plane_line(first, second, dist_floor)? {
        match planar {
            PlanarIntersection::Empty => {}
            PlanarIntersection::Overlap(region) => {
                components.push(SurfaceSurfaceComponent::Overlap(region))
            }
            PlanarIntersection::Curve(branch) => {
                components.push(SurfaceSurfaceComponent::Exact(branch))
            }
        }
        return Ok(SurfaceSurfaceIntersection {
            components,
            unresolved,
            boxes_visited: boxes_visited.max(1),
            bernstein_excluded,
            krawczyk_isolated,
            tolerance,
        });
    }
    // Exact iso reduction for a freeform chart against an affine plane carrier.
    if let Some((n, o, _, _)) = affine_plane(second)?
        && let Some(iso) = surface_plane_iso_components(first, n, o, second, dist_floor)?
    {
        return Ok(SurfaceSurfaceIntersection {
            components: iso
                .into_iter()
                .map(SurfaceSurfaceComponent::Exact)
                .collect(),
            unresolved,
            boxes_visited: boxes_visited.max(1),
            bernstein_excluded,
            krawczyk_isolated,
            tolerance,
        });
    }
    if let Some((n, o, _, _)) = affine_plane(first)?
        && let Some(mut iso) = surface_plane_iso_components(second, n, o, first, dist_floor)?
    {
        for component in &mut iso {
            component.swap_supports();
        }
        return Ok(SurfaceSurfaceIntersection {
            components: iso
                .into_iter()
                .map(SurfaceSurfaceComponent::Exact)
                .collect(),
            unresolved,
            boxes_visited: boxes_visited.max(1),
            bernstein_excluded,
            krawczyk_isolated,
            tolerance,
        });
    }

    let cells_a = surface_spans(first)?;
    let cells_b = surface_spans(second)?;
    check(
        cells_a.len().saturating_mul(cells_b.len()) <= MAX_SPANS,
        "Surface span-pair resource exceeded",
    )?;

    let mut pending: std::collections::VecDeque<SurfaceCellPending> = cells_a
        .into_iter()
        .flat_map(|a| {
            cells_b
                .iter()
                .copied()
                .map(move |b| (a, b, None, None, 0_usize))
        })
        .collect();

    // Unified guard as runaway insurance (item 1065): total pops are bounded
    // by initial cells plus two children per counted box, so the guard only
    // fires if the bookkeeping above is ever broken. MAX_BOXES itself keeps
    // its existing unresolved-entry semantics.
    let mut guard = Budget::new(MAX_SPANS + 2 * MAX_BOXES + 1, 37, u64::MAX)?
        .guard("ssi_subdivision");
    while let Some((uv, st, ha, hb, depth)) = pending.pop_front() {
        guard.tick()?;
        guard.check()?;
        if boxes_visited >= MAX_BOXES {
            unresolved.push(UnresolvedSurfaceIntersection {
                parameter_box: Some([uv[0], uv[1], uv[2], uv[3], st[0], st[1], st[2], st[3]]),
                reason: UnresolvedReason::ResourceBoundary,
                classification: None,
            });
            continue;
        }
        boxes_visited += 1;
        let (ha, hb) = match (ha, hb) {
            (Some(ha), Some(hb)) => (ha, hb),
            _ => {
                let pa = first.trim(uv)?;
                let pb = second.trim(st)?;
                (homogeneous_grid(&pa), homogeneous_grid(&pb))
            }
        };
        if grids_excluded_ss(&ha, &hb) {
            bernstein_excluded += 1;
            continue;
        }
        if proportional_grids(&ha, &hb, dist_floor) {
            components.push(SurfaceSurfaceComponent::Overlap(OverlapRegion {
                first_uv_box: uv,
                second_uv_box: st,
                geometry_enclosure: enclosure_of(
                    point3(
                        &first
                            .evaluate((uv[0] + uv[1]) * 0.5, (uv[2] + uv[3]) * 0.5)?
                            .point,
                    )?,
                    dist_floor,
                ),
                coedge_trim: coedge_trim([uv[0], uv[1]], [st[0], st[1]], [[0, 0], [0, 0]]),
            }));
            continue;
        }
        let width = (uv[1] - uv[0])
            .max(uv[3] - uv[2])
            .max(st[1] - st[0])
            .max(st[3] - st[2]);
        if width <= floor || depth >= 36 {
            let seed_uv = [(uv[0] + uv[1]) * 0.5, (uv[2] + uv[3]) * 0.5];
            let seed_st = [(st[0] + st[1]) * 0.5, (st[2] + st[3]) * 0.5];
            let pa = point3(&first.evaluate(seed_uv[0], seed_uv[1])?.point)?;
            let pb = point3(&second.evaluate(seed_st[0], seed_st[1])?.point)?;
            let residual = distance(&pa, &pb);
            let flat_a: Vec<[f64; 4]> = ha.iter().flatten().copied().collect();
            let flat_b: Vec<[f64; 4]> = hb.iter().flatten().copied().collect();
            let diag = next_up(hull_diagonal(&flat_a) + hull_diagonal(&flat_b));
            if residual - diag > dist_floor {
                continue;
            }
            match refine_seed(first, second, seed_uv, seed_st, uv, st, dist_floor)? {
                Some((ruv, rst, point, contact)) => {
                    if contact == ContactClass::PoleOrSingular {
                        unresolved.push(UnresolvedSurfaceIntersection {
                            parameter_box: Some([
                                uv[0], uv[1], uv[2], uv[3], st[0], st[1], st[2], st[3],
                            ]),
                            reason: UnresolvedReason::ConditioningBoundary,
                            classification: Some(UnresolvedClassification::PoleOrSingular),
                        });
                        continue;
                    }
                    if near_seed(&used_seeds, ruv, rst, floor) {
                        continue;
                    }
                    used_seeds.push((ruv, rst));
                    krawczyk_isolated += 1;
                    if matches!(
                        contact,
                        ContactClass::EvenTangency | ContactClass::OddTangency
                    ) {
                        components.push(SurfaceSurfaceComponent::Tangency(TangencyBranch {
                            contact,
                            sample: ContinuationSample {
                                point,
                                uv_first: ruv,
                                uv_second: rst,
                                parameter: 0.,
                                seam_wrap: None,
                            },
                            geometry_enclosure: enclosure_of(point, dist_floor),
                            coedge_trim: coedge_trim(
                                [ruv[0], ruv[0]],
                                [rst[0], rst[0]],
                                [[0, 0], [0, 0]],
                            ),
                            junction: false,
                        }));
                    } else {
                        let branch =
                            continue_branch(first, second, ruv, rst, point, contact, dist_floor)?;
                        components.push(SurfaceSurfaceComponent::Continued(branch));
                    }
                }
                None => {
                    if residual <= dist_floor {
                        unresolved.push(UnresolvedSurfaceIntersection {
                            parameter_box: Some([
                                uv[0], uv[1], uv[2], uv[3], st[0], st[1], st[2], st[3],
                            ]),
                            reason: UnresolvedReason::ConditioningBoundary,
                            classification: None,
                        });
                    } else {
                        unresolved.push(UnresolvedSurfaceIntersection {
                            parameter_box: Some([
                                uv[0], uv[1], uv[2], uv[3], st[0], st[1], st[2], st[3],
                            ]),
                            reason: UnresolvedReason::ConditioningBoundary,
                            classification: Some(
                                UnresolvedClassification::NearMissOrIllConditioned,
                            ),
                        });
                    }
                }
            }
            continue;
        }
        // Split the largest of the four axes.
        let widths = [uv[1] - uv[0], uv[3] - uv[2], st[1] - st[0], st[3] - st[2]];
        let axis = widths
            .iter()
            .enumerate()
            .max_by(|a, b| a.1.total_cmp(b.1))
            .map(|(i, _)| i)
            .unwrap_or(0);
        match axis {
            0 => {
                let mid = (uv[0] + uv[1]) * 0.5;
                let (l, r) = split_grid_u(&ha);
                pending.push_back((
                    [uv[0], mid, uv[2], uv[3]],
                    st,
                    Some(l),
                    Some(hb.clone()),
                    depth + 1,
                ));
                pending.push_back(([mid, uv[1], uv[2], uv[3]], st, Some(r), Some(hb), depth + 1));
            }
            1 => {
                let mid = (uv[2] + uv[3]) * 0.5;
                let (l, r) = split_grid_v(&ha);
                pending.push_back((
                    [uv[0], uv[1], uv[2], mid],
                    st,
                    Some(l),
                    Some(hb.clone()),
                    depth + 1,
                ));
                pending.push_back(([uv[0], uv[1], mid, uv[3]], st, Some(r), Some(hb), depth + 1));
            }
            2 => {
                let mid = (st[0] + st[1]) * 0.5;
                let (l, r) = split_grid_u(&hb);
                pending.push_back((
                    uv,
                    [st[0], mid, st[2], st[3]],
                    Some(ha.clone()),
                    Some(l),
                    depth + 1,
                ));
                pending.push_back((uv, [mid, st[1], st[2], st[3]], Some(ha), Some(r), depth + 1));
            }
            _ => {
                let mid = (st[2] + st[3]) * 0.5;
                let (l, r) = split_grid_v(&hb);
                pending.push_back((
                    uv,
                    [st[0], st[1], st[2], mid],
                    Some(ha.clone()),
                    Some(l),
                    depth + 1,
                ));
                pending.push_back((uv, [st[0], st[1], mid, st[3]], Some(ha), Some(r), depth + 1));
            }
        }
    }

    if components.len() > MAX_BRANCHES {
        unresolved.push(UnresolvedSurfaceIntersection {
            parameter_box: None,
            reason: UnresolvedReason::ResourceBoundary,
            classification: Some(UnresolvedClassification::BranchBudget),
        });
        components.truncate(MAX_BRANCHES);
    }

    // Detect junctions: shared endpoints among distinct curve components.
    mark_junctions(&mut components, floor);

    Ok(SurfaceSurfaceIntersection {
        components,
        unresolved,
        boxes_visited: boxes_visited,
        bernstein_excluded,
        krawczyk_isolated,
        tolerance,
    })
}

#[cfg(test)]
mod guard_tests {
    use super::*;
    use crate::foundation::guards::Budget;

    fn plane(z: f64) -> Surface {
        Surface {
            degree_u: 1,
            degree_v: 1,
            knots_u: vec![0., 0., 1., 1.],
            knots_v: vec![0., 0., 1., 1.],
            weights: vec![vec![1.; 2]; 2],
            control_points: vec![
                vec![vec![0., 0., z], vec![0., 1., z]],
                vec![vec![1., 0., z], vec![1., 1., z]],
            ],
            periodic_u: false,
            periodic_v: false,
        }
    }

    #[test]
    fn nan_control_point_is_rejected_at_the_boundary() {
        let a = plane(0.);
        let mut b = plane(0.5);
        b.control_points[0][0][2] = f64::NAN;
        let err = intersect_surface_surface_report(&a, &b, None).err().expect("invalid intersection input must fail");
        assert_eq!(err.code, crate::INVALID_INPUT, "{err:?}");
    }

    #[test]
    fn crossing_planes_resolve_with_guards_active() {
        // Plane z=0 against the tilted plane z=x-0.5: one exact branch.
        let a = plane(0.);
        let mut b = plane(0.);
        for row in &mut b.control_points {
            for p in row {
                p[2] = p[0] - 0.5;
                p[0] = 0.5;
            }
        }
        let r = intersect_surface_surface_report(&a, &b, None).unwrap();
        assert!(!r.components.is_empty());
    }

    #[test]
    fn marching_guard_matches_continuation_cap() {
        let mut guard =
            Budget::with_iterations(MAX_CONTINUATION).unwrap().guard("ssi_marching");
        for _ in 0..MAX_CONTINUATION {
            guard.tick().unwrap();
        }
        let err = guard.tick().unwrap_err();
        assert_eq!(err.code, crate::RESOURCE_LIMIT, "{err:?}");
        assert!(err.contains("ssi_marching"));
    }
}
