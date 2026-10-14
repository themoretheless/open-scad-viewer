use super::*;

/// Solve only an admitted, numerically stable frame model. Errors do not carry
/// successful-looking displacement, reaction, or diagram values.
pub(super) fn solve_case(a: &Assembled, load_set: &LoadSet) -> Result<Response> {
    let forces = prepare_loads(&a.beams, load_set, a.n)?;
    let n = a.n;
    let ndof = n * 6;
    let mut load = DVector::<f64>::zeros(ndof);
    for i in 0..n {
        for c in 0..3 {
            load[i * 6 + c] = load_set.forces_n[i][c];
            load[i * 6 + 3 + c] = load_set.moments_nmm[i][c];
        }
    }
    let global_dof = |beam: &Beam, local: usize| beam.nodes[local / 6] * 6 + local % 6;
    for (beam, bf) in a.beams.iter().zip(&forces) {
        let t = transformation(&beam.rotation);
        let mut f_full = DVector::<f64>::zeros(12);
        for (i, &di) in beam.kept.iter().enumerate() {
            f_full[di] = bf.f_eff[i];
        }
        let f_glob = &t * &f_full;
        if f_glob.iter().any(|v| !v.is_finite()) {
            return Err(numeric());
        }
        for d1 in 0..12 {
            load[global_dof(beam, d1)] += f_glob[d1];
        }
    }
    let restrained = |dof: usize| a.restrained[dof / 6][dof % 6];
    let mut displacement = DVector::<f64>::zeros(ndof);
    if let Some(factor) = &a.factor {
        let rhs = DVector::from_iterator(
            a.free.len(),
            a.free
                .iter()
                .enumerate()
                .map(|(i, &dof)| load[dof] / a.scales[i]),
        );
        if rhs.iter().any(|v| !v.is_finite()) {
            return Err(numeric());
        }
        let solved = factor.solve(&rhs);
        for (i, &dof) in a.free.iter().enumerate() {
            displacement[dof] = solved[i] / a.scales[i];
        }
        if displacement.iter().any(|v| !v.is_finite()) {
            return Err(numeric());
        }
    }

    // Normwise backward error and reactions, same policy as the truss solver.
    let displacement_norm = displacement.iter().map(|x| x.abs()).fold(0., f64::max);
    let force_norm = load.iter().map(|x| x.abs()).fold(0., f64::max);
    let system_scale = a.matrix_norm * displacement_norm + force_norm;
    if !system_scale.is_finite() {
        return Err(numeric());
    }
    let mut reaction = DVector::<f64>::zeros(ndof);
    let mut max_relative_residual = 0f64;
    for i in 0..ndof {
        let mut residual = -load[i];
        let mut scale = load[i].abs();
        for j in 0..ndof {
            let contribution = a.stiffness[(i, j)] * displacement[j];
            residual += contribution;
            scale += contribution.abs();
        }
        if !residual.is_finite() || !scale.is_finite() {
            return Err(numeric());
        }
        if restrained(i) {
            reaction[i] = residual;
        } else {
            let relative = if system_scale == 0. {
                0.
            } else {
                residual.abs() / system_scale
            };
            max_relative_residual = max_relative_residual.max(relative);
        }
    }
    if max_relative_residual > MAX_RELATIVE_RESIDUAL {
        return Err(Error::new(
            "FRAME_RESIDUAL",
            "Frame solution does not satisfy free-DOF equilibrium",
        ));
    }

    // Recover member end forces and section diagrams in local axes.
    let t_cache: Vec<DMatrix<f64>> = a
        .beams
        .iter()
        .map(|b| transformation(&b.rotation))
        .collect();
    let mut member_results = Vec::with_capacity(a.beams.len());
    for ((beam, bf), t) in a.beams.iter().zip(&forces).zip(&t_cache) {
        let mut u_glob = DVector::<f64>::zeros(12);
        for d in 0..12 {
            u_glob[d] = displacement[global_dof(beam, d)];
        }
        let mut u_local = t.transpose() * u_glob;
        if !beam.released.is_empty() {
            let chol = beam.k_rr.as_ref().expect("validated condensation");
            let u_k = subvector(&u_local, &beam.kept);
            let f_r = subvector(&bf.f_eq, &beam.released);
            let u_r = chol.solve(&(f_r - &beam.k_rk * u_k));
            for (i, &d) in beam.released.iter().enumerate() {
                u_local[d] = u_r[i];
            }
        }
        // Force the element exerts back on its nodes, local axes; the equal
        // and opposite force is what node A applies to the element end.
        let q = &beam.k_local * &u_local - &bf.f_eq;
        let mut end_on_element = [0.; 6];
        for (c, value) in end_on_element.iter_mut().enumerate() {
            *value = q[c];
        }
        let stations: Vec<Station> = (0..DIAGRAM_STATIONS)
            .map(|i| {
                station_resultants(
                    beam,
                    bf,
                    &end_on_element,
                    beam.length * i as f64 / (DIAGRAM_STATIONS - 1) as f64,
                )
            })
            .collect();
        if stations.iter().any(|s| {
            ![
                s.axial_n,
                s.shear_y_n,
                s.shear_z_n,
                s.torsion_nmm,
                s.moment_y_nmm,
                s.moment_z_nmm,
            ]
            .iter()
            .all(|v| v.is_finite())
        }) {
            return Err(numeric());
        }
        member_results.push(MemberResult { stations });
    }

    let displacements_mm: Vec<[f64; 3]> = (0..n)
        .map(|i| std::array::from_fn(|k| displacement[i * 6 + k]))
        .collect();
    let rotations_rad: Vec<[f64; 3]> = (0..n)
        .map(|i| std::array::from_fn(|k| displacement[i * 6 + 3 + k]))
        .collect();
    let reactions_n: Vec<[f64; 3]> = (0..n)
        .map(|i| std::array::from_fn(|k| reaction[i * 6 + k]))
        .collect();
    let reaction_moments_nmm: Vec<[f64; 3]> = (0..n)
        .map(|i| std::array::from_fn(|k| reaction[i * 6 + 3 + k]))
        .collect();
    let max_deflection_mm = displacements_mm
        .iter()
        .map(|d| d[0].hypot(d[1]).hypot(d[2]))
        .fold(0., f64::max);
    if !max_deflection_mm.is_finite() {
        return Err(numeric());
    }
    Ok(Response {
        displacements_mm,
        rotations_rad,
        reactions_n,
        reaction_moments_nmm,
        members: member_results,
        max_deflection_mm,
        max_relative_residual,
        free_dofs: a.free.len(),
    })
}

/// Validate support conditions against the structure and rigid restraints.
pub(super) fn validate_supports(supports: &[Support], n: usize, restrained: &[[bool; 6]]) -> Result<()> {
    if supports.len() > MAX_SUPPORTS {
        return Err(invalid("Supports are capped at 6 per node"));
    }
    let mut seen = BTreeSet::new();
    for support in supports {
        let (node, dof) = support.node_dof();
        if node >= n || dof >= 6 {
            return Err(invalid("Support references an unknown node or DOF"));
        }
        if restrained[node][dof] {
            return Err(invalid("Support duplicates a rigid restraint"));
        }
        if !seen.insert((node, dof)) {
            return Err(invalid("Duplicate support on one DOF"));
        }
        if let Some(k) = support.stiffness() {
            if !k.is_finite() || k <= 0. {
                return Err(invalid("Spring stiffness must be finite and positive"));
            }
        }
    }
    Ok(())
}

/// Reaction a support exerts on the structure, read back from a response.
pub(super) fn support_reaction(r: &Response, node: usize, dof: usize) -> f64 {
    if dof < 3 {
        r.reactions_n[node][dof]
    } else {
        r.reaction_moments_nmm[node][dof - 3]
    }
}

pub(super) fn support_displacement(r: &Response, node: usize, dof: usize) -> f64 {
    if dof < 3 {
        r.displacements_mm[node][dof]
    } else {
        r.rotations_rad[node][dof - 3]
    }
}

pub(super) fn set_support_reaction(r: &mut Response, node: usize, dof: usize, value: f64) {
    if dof < 3 {
        r.reactions_n[node][dof] = value;
    } else {
        r.reaction_moments_nmm[node][dof - 3] = value;
    }
}

/// Solve one load set with springs and unilateral contacts. Unilateral
/// supports iterate an active set: engaged bounds contribute their DOF to the
/// restraint mask, engaged unilateral springs contribute stiffness, and the
/// set is updated from reaction signs (release) and penetrations (engagement)
/// until it stabilizes.
pub(super) fn solve_supported(
    nodes_mm: &[[f64; 3]],
    members: &[Member],
    restrained: &[[bool; 6]],
    supports: &[Support],
    load_set: &LoadSet,
) -> Result<Response> {
    let n = nodes_mm.len();
    validate_supports(supports, n, restrained)?;
    if supports.is_empty() {
        let assembled = assemble(nodes_mm, members, restrained, &[])?;
        return solve_case(&assembled, load_set);
    }
    // Every unilateral support starts engaged (contact assumed): engagement
    // never destabilizes, while a support left out of the first solve could
    // leave the structure a mechanism. Disengagement follows from reaction
    // signs, re-engagement from penetration.
    let mut active: Vec<bool> = vec![true; supports.len()];
    let max_iterations = 6 + 2 * supports.len();
    let mut mask;
    let mut diagonal: Vec<(usize, f64)> = Vec::new();
    let mut iteration = 0;
    let response = loop {
        iteration += 1;
        if iteration > max_iterations {
            return Err(Error::new(
                "FRAME_NO_CONVERGENCE",
                "Unilateral support active set did not stabilize",
            ));
        }
        mask = restrained.to_vec();
        diagonal.clear();
        for (support, &on) in supports.iter().zip(&active) {
            let (node, dof) = support.node_dof();
            match (*support, on) {
                (Support::Spring { stiffness, .. }, _) => {
                    diagonal.push((node * 6 + dof, stiffness));
                }
                (Support::LowerBound { .. } | Support::UpperBound { .. }, true) => {
                    mask[node][dof] = true;
                }
                (Support::LowerSpring { stiffness, .. } | Support::UpperSpring { stiffness, .. }, true) => {
                    diagonal.push((node * 6 + dof, stiffness));
                }
                _ => {}
            }
        }
        let assembled = assemble(nodes_mm, members, &mask, &diagonal)?;
        let r = solve_case(&assembled, load_set)?;
        if !supports.iter().any(|s| s.is_unilateral()) {
            break r;
        }
        let u_scale = r
            .displacements_mm
            .iter()
            .chain(&r.rotations_rad)
            .flatten()
            .map(|v| v.abs())
            .fold(0., f64::max);
        let r_scale = r
            .reactions_n
            .iter()
            .chain(&r.reaction_moments_nmm)
            .flatten()
            .map(|v| v.abs())
            .fold(0., f64::max);
        let u_tol = 1e-9 * (1. + u_scale);
        let r_tol = 1e-9 * (1. + r_scale);
        let mut changed = false;
        for (i, support) in supports.iter().enumerate() {
            let (node, dof) = support.node_dof();
            let u = support_displacement(&r, node, dof);
            let reaction = support_reaction(&r, node, dof);
            let next = match *support {
                Support::LowerBound { .. } => {
                    if active[i] {
                        reaction >= -r_tol // prop cannot pull: release
                    } else {
                        u < -u_tol // penetration: re-engage
                    }
                }
                Support::UpperBound { .. } => {
                    if active[i] {
                        reaction <= r_tol
                    } else {
                        u > u_tol
                    }
                }
                Support::LowerSpring { .. } => u < -u_tol,
                Support::UpperSpring { .. } => u > u_tol,
                Support::Spring { .. } => active[i],
            };
            if next != active[i] {
                active[i] = next;
                changed = true;
            }
        }
        if !changed {
            break r;
        }
    };
    let mut response = response;
    // Report spring forces as support reactions (spring DOFs are free in the
    // factorization, so the residual there is zero by construction).
    for (support, &on) in supports.iter().zip(&active) {
        let (node, dof) = support.node_dof();
        let k = match support {
            Support::Spring { stiffness, .. } => Some(*stiffness),
            Support::LowerSpring { stiffness, .. } | Support::UpperSpring { stiffness, .. } => {
                on.then_some(*stiffness)
            }
            _ => None,
        };
        if let Some(k) = k {
            let u = support_displacement(&response, node, dof);
            set_support_reaction(&mut response, node, dof, -k * u);
        }
    }
    Ok(response)
}

pub fn solve(model: &Model) -> Result<Response> {
    solve_supported(
        &model.nodes_mm,
        &model.members,
        &model.restrained,
        &model.supports,
        &LoadSet {
            forces_n: model.forces_n.clone(),
            moments_nmm: model.moments_nmm.clone(),
            loads: model.loads.clone(),
        },
    )
}

const STATION_SCALARS: usize = 6;

/// Flat scalar layout for envelopes: per node [u, r, R, M] (12), then per
/// member per station [N, Vy, Vz, T, My, Mz]. Station positions are excluded
/// (constant for a given structure).
pub(super) fn flatten_response(r: &Response, n: usize) -> Vec<f64> {
    let mut out =
        Vec::with_capacity(n * 12 + r.members.len() * DIAGRAM_STATIONS * STATION_SCALARS);
    for i in 0..n {
        out.extend_from_slice(&r.displacements_mm[i]);
        out.extend_from_slice(&r.rotations_rad[i]);
        out.extend_from_slice(&r.reactions_n[i]);
        out.extend_from_slice(&r.reaction_moments_nmm[i]);
    }
    for member in &r.members {
        for s in &member.stations {
            out.extend_from_slice(&[
                s.axial_n,
                s.shear_y_n,
                s.shear_z_n,
                s.torsion_nmm,
                s.moment_y_nmm,
                s.moment_z_nmm,
            ]);
        }
    }
    out
}

/// Scale a member load's vectors by a combination factor; position and axes
/// are preserved, so a scaled load remains an admitted load.
pub(super) fn scale_member_load(load: &MemberLoad, factor: f64) -> MemberLoad {
    let s = |v: [f64; 3]| [v[0] * factor, v[1] * factor, v[2] * factor];
    match *load {
        MemberLoad::PointForce {
            member,
            at_mm,
            force_n,
            local_axes,
        } => MemberLoad::PointForce {
            member,
            at_mm,
            force_n: s(force_n),
            local_axes,
        },
        MemberLoad::PointMoment {
            member,
            at_mm,
            moment_nmm,
            local_axes,
        } => MemberLoad::PointMoment {
            member,
            at_mm,
            moment_nmm: s(moment_nmm),
            local_axes,
        },
        MemberLoad::Uniform {
            member,
            force_n_per_mm,
            local_axes,
        } => MemberLoad::Uniform {
            member,
            force_n_per_mm: s(force_n_per_mm),
            local_axes,
        },
        MemberLoad::Trapezoidal {
            member,
            from_n_per_mm,
            to_n_per_mm,
            local_axes,
        } => MemberLoad::Trapezoidal {
            member,
            from_n_per_mm: s(from_n_per_mm),
            to_n_per_mm: s(to_n_per_mm),
            local_axes,
        },
    }
}

/// Merge load cases into one combination load set: nodal loads scale
/// elementwise, member loads scale their vectors and concatenate.
pub(super) fn combine_load_sets(cases: &[LoadSet], factors: &[f64], n: usize) -> Result<LoadSet> {
    let mut forces_n = vec![[0.; 3]; n];
    let mut moments_nmm = vec![[0.; 3]; n];
    let mut loads = Vec::new();
    for (case, &factor) in cases.iter().zip(factors) {
        if factor == 0. {
            continue;
        }
        for i in 0..n {
            for c in 0..3 {
                forces_n[i][c] += factor * case.forces_n[i][c];
                moments_nmm[i][c] += factor * case.moments_nmm[i][c];
            }
        }
        for load in &case.loads {
            loads.push(scale_member_load(load, factor));
        }
    }
    if loads.len() > MAX_COMBINED_MEMBER_LOADS {
        return Err(invalid("Combined member loads exceed the 4096 budget"));
    }
    if forces_n.iter().chain(&moments_nmm).any(|v| !finite3(v)) {
        return Err(numeric());
    }
    Ok(LoadSet {
        forces_n,
        moments_nmm,
        loads,
    })
}

/// Solve load cases against one shared structure and envelope the responses
/// across combinations. Two exact paths: with only rigid restraints and
/// bidirectional springs the response is linear, so the stiffness is factored
/// once and combinations are formed from the case responses; with unilateral
/// supports superposition does not hold, so every combination is merged into
/// one load set and solved directly with the active-set iteration. Errors
/// from any case abort the whole request; envelopes never mix with
/// successful-looking partial data.
pub fn solve_envelopes(
    nodes_mm: &[[f64; 3]],
    members: &[Member],
    restrained: &[[bool; 6]],
    supports: &[Support],
    cases: &[LoadSet],
    combinations: &[Combination],
) -> Result<EnvelopeResponse> {
    crate::combos::validate(combinations, cases.len())?;
    let n = nodes_mm.len();
    validate_supports(supports, n, restrained)?;
    let width = n * 12 + members.len() * DIAGRAM_STATIONS * STATION_SCALARS;
    // One flat response vector per combination, exact in both paths.
    let mut combined_flat: Vec<Vec<f64>> = Vec::with_capacity(combinations.len());
    let mut max_relative_residual = 0f64;
    let grid_response: Response;
    if supports.iter().any(|s| s.is_unilateral()) {
        let mut grid = None;
        for combination in combinations {
            let merged = combine_load_sets(cases, &combination.factors, n)?;
            let r = solve_supported(nodes_mm, members, restrained, supports, &merged)?;
            max_relative_residual = max_relative_residual.max(r.max_relative_residual);
            if grid.is_none() {
                grid = Some(r.clone());
            }
            combined_flat.push(flatten_response(&r, n));
        }
        grid_response = grid.expect("validated nonempty combinations");
    } else {
        let diagonal: Vec<(usize, f64)> = supports
            .iter()
            .map(|s| {
                let (node, dof) = s.node_dof();
                (node * 6 + dof, s.stiffness().expect("bidirectional spring"))
            })
            .collect();
        let assembled = assemble(nodes_mm, members, restrained, &diagonal)?;
        let responses = cases
            .iter()
            .map(|case| solve_case(&assembled, case))
            .collect::<Result<Vec<_>>>()?;
        max_relative_residual = responses
            .iter()
            .map(|r| r.max_relative_residual)
            .fold(0., f64::max);
        let flat: Vec<Vec<f64>> = responses.iter().map(|r| flatten_response(r, n)).collect();
        for combination in combinations {
            let series: Vec<&[f64]> = flat.iter().map(Vec::as_slice).collect();
            combined_flat.push(crate::combos::combine_series(&series, &combination.factors)?);
        }
        grid_response = responses.into_iter().next().expect("validated nonempty cases");
    }
    let mut envelopes: Vec<MinMax> = Vec::with_capacity(width);
    let mut deflection: Option<MinMax> = None;
    for (ci, combined) in combined_flat.iter().enumerate() {
        if ci == 0 {
            envelopes = combined.iter().map(|&v| MinMax::of(v, 0)).collect();
        } else {
            for (e, &v) in envelopes.iter_mut().zip(combined) {
                e.absorb(v, ci);
            }
        }
        let mut combo_max_deflection = 0f64;
        for node in 0..n {
            let base = node * 12;
            combo_max_deflection = combo_max_deflection.max(
                combined[base]
                    .hypot(combined[base + 1])
                    .hypot(combined[base + 2]),
            );
        }
        if !combo_max_deflection.is_finite() {
            return Err(numeric());
        }
        match &mut deflection {
            None => deflection = Some(MinMax::of(combo_max_deflection, ci)),
            Some(e) => e.absorb(combo_max_deflection, ci),
        }
    }
    let node_env = |offset: usize| -> Vec<[MinMax; 3]> {
        (0..n)
            .map(|i| std::array::from_fn(|k| envelopes[i * 12 + offset + k]))
            .collect()
    };
    let members_env: Vec<Vec<StationEnvelope>> = (0..members.len())
        .map(|mi| {
            (0..DIAGRAM_STATIONS)
                .map(|si| {
                    let b = n * 12 + (mi * DIAGRAM_STATIONS + si) * STATION_SCALARS;
                    StationEnvelope {
                        x_mm: grid_response.members[mi].stations[si].x_mm,
                        axial_n: envelopes[b],
                        shear_y_n: envelopes[b + 1],
                        shear_z_n: envelopes[b + 2],
                        torsion_nmm: envelopes[b + 3],
                        moment_y_nmm: envelopes[b + 4],
                        moment_z_nmm: envelopes[b + 5],
                    }
                })
                .collect()
        })
        .collect();
    Ok(EnvelopeResponse {
        load_cases: cases.len(),
        combinations: combinations.iter().map(|c| c.name.clone()).collect(),
        displacements_mm: node_env(0),
        rotations_rad: node_env(3),
        reactions_n: node_env(6),
        reaction_moments_nmm: node_env(9),
        members: members_env,
        max_deflection_mm: deflection.expect("validated nonempty combinations"),
        max_relative_residual,
        free_dofs: grid_response.free_dofs,
    })
}
