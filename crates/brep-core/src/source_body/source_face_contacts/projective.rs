use super::*;
/// Inspect every face pair with exact original hull separation and shared-line
/// confinement. Unsupported contacts fall back to the original UV-domain
/// search and remain unresolved unless that search proves absence.
pub fn inspect_shell_with_line_contacts<
    E: crate::source_allowed_contact::Support + crate::source_vertex_contact::VertexCarrier,
>(
    shell: &crate::source_shell_incidence::Shell<E>,
    tolerance_uv: f64,
    limits: crate::face_contacts::Limits,
    max_work: u64,
    max_spans: usize,
) -> Result<ShellReport<E>> {
    inspect_shell_with_projective_fibers(shell, tolerance_uv, limits, max_work, max_spans, 0, 0)
}
/// Fresh boundary-fiber proofs share global exact, span and driver budgets.
/// Unsupported contact remains unresolved and falls back to domain search.
pub fn inspect_shell_with_projective_fibers<
    E: crate::source_allowed_contact::Support + crate::source_vertex_contact::VertexCarrier,
>(
    shell: &crate::source_shell_incidence::Shell<E>,
    tolerance_uv: f64,
    limits: crate::face_contacts::Limits,
    max_work: u64,
    max_spans: usize,
    max_driver: usize,
    max_linear: usize,
) -> Result<ShellReport<E>> {
    if !(1..=100000).contains(&limits.pairs)
        || !(1..=1000000).contains(&limits.cells)
        || !(1..=8000000).contains(&limits.domain_cells)
        || !(1..=100000).contains(&limits.cells_per_pair)
        || !(1..=1000000).contains(&limits.domain_cells_per_pair)
        || !tolerance_uv.is_finite()
        || tolerance_uv <= 0.
        || !(1..=100000000).contains(&max_work)
        || !(1..=100000).contains(&max_spans)
        || max_driver > 100000
        || max_linear > 100000
    {
        return Err(Error::new(
            "BREP_SOURCE_CONTACT",
            "Choose bounded exact contact and source search work",
        ));
    }
    let regions = shell.regions().ok_or_else(|| {
        Error::new(
            "BREP_SOURCE_CONTACT",
            "Require qualified original material regions",
        )
    })?;
    let n = regions.len();
    let mut out = ShellReport {
        pairs: Vec::new(),
        total_pairs: n * (n - 1) / 2,
        next_pair: None,
        cells: 0,
        domain_cells: 0,
        all_pairs_absence_proven: true,
        all_pairs_qualified: true,
        exact_work: 0,
        spans: 0,
        driver_cells: 0,
        linear_cells: 0,
    };
    for a in 0..n {
        for b in a + 1..n {
            if out.pairs.len() == limits.pairs
                || out.cells == limits.cells
                || out.domain_cells == limits.domain_cells
            {
                out.next_pair = Some([a, b]);
                out.all_pairs_absence_proven = false;
                out.all_pairs_qualified = false;
                return Ok(out);
            }
            let mut pair = PairReport {
                faces: [a, b],
                result: None,
                allowed: None,
                fiber: None,
                interior_fiber: None,
                paired_fiber: None,
                paired_projection: None,
                curved_boundary: None,
                vertex_contact: None,
                ruled_projection: None,
                pole_planar: None,
                pole_paired: None,
                disjoint_hull: None,
            };
            if out.exact_work < max_work {
                let r = crate::source_hull_separation::certify(
                    [&regions[a], &regions[b]],
                    max_work - out.exact_work,
                )?;
                out.exact_work += r.exact_work;
                if let Some(proof) = r.certificate {
                    pair.disjoint_hull = Some(proof);
                    out.pairs.push(pair);
                    continue;
                }
            }
            if out.exact_work < max_work && out.spans < max_spans {
                let r = crate::source_allowed_contact::certify(
                    shell,
                    [a, b],
                    max_work - out.exact_work,
                    max_spans - out.spans,
                )?;
                out.exact_work += r.exact_work;
                out.spans += r.spans;
                if let Some(proof) = r.certificate {
                    pair.allowed = Some(proof);
                    out.all_pairs_absence_proven = false;
                    out.pairs.push(pair);
                    continue;
                }
            }
            if max_driver > 0 {
                let mut qualified = false;
                for faces in [[a, b], [b, a]] {
                    if out.exact_work == max_work
                        || out.spans == max_spans
                        || out.driver_cells == max_driver
                    {
                        break;
                    }
                    let r = crate::source_fiber_contact::certify_with_linear_chart(
                        shell,
                        faces,
                        max_work - out.exact_work,
                        max_spans - out.spans,
                        max_driver - out.driver_cells,
                        max_linear - out.linear_cells,
                    )?;
                    out.exact_work += r.exact_work;
                    out.spans += r.spans;
                    out.driver_cells += r.driver_cells;
                    out.linear_cells += r.linear_cells;
                    if let Some(proof) = r.certificate {
                        pair.fiber = Some(proof);
                        qualified = true;
                        break;
                    }
                }
                if !qualified {
                    for faces in [[a, b], [b, a]] {
                        if out.exact_work == max_work
                            || out.spans == max_spans
                            || out.driver_cells == max_driver
                        {
                            break;
                        }
                        let r = crate::source_interior_contact::certify_with_linear_chart(
                            shell,
                            faces,
                            max_work - out.exact_work,
                            max_spans - out.spans,
                            max_driver - out.driver_cells,
                            max_linear - out.linear_cells,
                        )?;
                        out.exact_work += r.exact_work;
                        out.spans += r.spans;
                        out.driver_cells += r.driver_cells;
                        out.linear_cells += r.linear_cells;
                        if let Some(proof) = r.certificate {
                            pair.interior_fiber = Some(proof);
                            qualified = true;
                            break;
                        }
                    }
                }
                if !qualified && out.exact_work < max_work && out.driver_cells < max_driver {
                    let r = crate::source_paired_fiber_contact::certify(
                        shell,
                        [a, b],
                        max_work - out.exact_work,
                        max_driver - out.driver_cells,
                    )?;
                    out.exact_work += r.exact_work;
                    out.driver_cells += r.driver_cells;
                    if let Some(proof) = r.certificate {
                        pair.paired_fiber = Some(proof);
                        qualified = true;
                    }
                }
                if !qualified && out.exact_work < max_work && out.driver_cells < max_driver {
                    let r = crate::source_pole_planar_contact::certify(
                        shell,
                        [a, b],
                        max_work - out.exact_work,
                        max_driver - out.driver_cells,
                    )?;
                    out.exact_work += r.exact_work;
                    out.driver_cells += r.driver_cells;
                    if let Some(proof) = r.certificate {
                        pair.pole_planar = Some(proof);
                        qualified = true;
                    }
                }
                if !qualified && out.exact_work < max_work && out.driver_cells < max_driver {
                    let r = crate::source_pole_paired_contact::certify(
                        shell,
                        [a, b],
                        max_work - out.exact_work,
                        max_driver - out.driver_cells,
                    )?;
                    out.exact_work += r.exact_work;
                    out.driver_cells += r.driver_cells;
                    if let Some(proof) = r.certificate {
                        pair.pole_paired = Some(proof);
                        qualified = true;
                    }
                }
                if qualified {
                    out.all_pairs_absence_proven = false;
                    out.pairs.push(pair);
                    continue;
                }
            }
            if out.exact_work < max_work {
                let r = crate::source_vertex_contact::certify(
                    shell,
                    [a, b],
                    max_work - out.exact_work,
                )?;
                out.exact_work += r.exact_work;
                if let Some(proof) = r.certificate {
                    pair.vertex_contact = Some(proof);
                    out.all_pairs_absence_proven = false;
                    out.pairs.push(pair);
                    continue;
                }
            }
            if out.exact_work < max_work && out.driver_cells < max_driver {
                let r = crate::source_paired_projection_contact::certify(
                    shell,
                    [a, b],
                    max_work - out.exact_work,
                    max_driver - out.driver_cells,
                )?;
                out.exact_work += r.exact_work;
                out.driver_cells += r.driver_cells;
                if let Some(proof) = r.certificate {
                    pair.paired_projection = Some(proof);
                    out.all_pairs_absence_proven = false;
                    out.pairs.push(pair);
                    continue;
                }
            }
            if out.exact_work < max_work && out.driver_cells < max_driver {
                let r = crate::source_curved_boundary_contact::certify(
                    shell,
                    [a, b],
                    max_work - out.exact_work,
                    max_driver - out.driver_cells,
                )?;
                out.exact_work += r.exact_work;
                out.driver_cells += r.driver_cells;
                if let Some(proof) = r.certificate {
                    pair.curved_boundary = Some(proof);
                    out.all_pairs_absence_proven = false;
                    out.pairs.push(pair);
                    continue;
                }
            }
            let result = search(
                [&regions[a], &regions[b]],
                tolerance_uv,
                limits.cells_per_pair.min(limits.cells - out.cells),
                limits
                    .domain_cells_per_pair
                    .min(limits.domain_cells - out.domain_cells),
            )?;
            out.cells += result.cells;
            out.domain_cells += result.domain_cells;
            out.all_pairs_absence_proven &= result.absence_proven;
            out.all_pairs_qualified &= result.absence_proven;
            pair.result = Some(result);
            out.pairs.push(pair);
        }
    }
    Ok(out)
}
