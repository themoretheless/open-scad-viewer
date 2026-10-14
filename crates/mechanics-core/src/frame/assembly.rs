use super::*;

/// Validate the shared structure and build load-independent member data,
/// including the static condensation factors of end releases.
pub(super) fn validate_structure(
    nodes_mm: &[[f64; 3]],
    members: &[Member],
    restrained: &[[bool; 6]],
) -> Result<Vec<Beam>> {
    let n = nodes_mm.len();
    if n == 0 || n > MAX_NODES || members.is_empty() || members.len() > MAX_MEMBERS {
        return Err(invalid("Frame requires 1-125 nodes and 1-400 members"));
    }
    if restrained.len() != n {
        return Err(invalid("Each node requires a restraint mask"));
    }
    if nodes_mm.iter().any(|v| !finite3(v)) {
        return Err(invalid("Node coordinates must be finite"));
    }
    let mut seen = BTreeSet::new();
    let mut beams = Vec::with_capacity(members.len());
    for member in members {
        let [a, b] = member.nodes;
        if a >= n || b >= n || a == b || !seen.insert([a.min(b), a.max(b)]) {
            return Err(invalid(
                "Members require distinct valid nodes and unique unordered edges",
            ));
        }
        let positive = [
            member.young_mpa,
            member.area_mm2,
            member.iyy_mm4,
            member.izz_mm4,
            member.j_mm4,
        ];
        if positive.iter().any(|v| !v.is_finite() || *v <= 0.)
            || !member.poisson.is_finite()
            || member.poisson <= -1.
            || member.poisson >= 0.5
        {
            return Err(invalid(
                "Member E, A, I, J must be finite and positive; Poisson in (-1, 0.5)",
            ));
        }
        for area in [member.shear_area_y_mm2, member.shear_area_z_mm2]
            .into_iter()
            .flatten()
        {
            if !area.is_finite() || area <= 0. {
                return Err(invalid("Shear areas must be finite and positive"));
            }
        }
        let pa = Vector3::from(nodes_mm[a]);
        let pb = Vector3::from(nodes_mm[b]);
        let delta = pb - pa;
        let length = delta.norm();
        if !length.is_finite() || length == 0. {
            return Err(invalid("Zero-length members are not supported"));
        }
        let rotation = local_frame(
            delta / length,
            member.local_z_hint.unwrap_or([0., 0., 1.]),
        )?;
        let e = member.young_mpa;
        let g = e / (2. * (1. + member.poisson));
        if !g.is_finite() || g <= 0. {
            return Err(numeric());
        }
        let k_local = beam_stiffness(
            e,
            g,
            member.area_mm2,
            member.iyy_mm4,
            member.izz_mm4,
            member.j_mm4,
            member.shear_area_y_mm2,
            member.shear_area_z_mm2,
            length,
        );
        if k_local.iter().any(|v| !v.is_finite()) {
            return Err(numeric());
        }
        let released: Vec<usize> = [
            (3usize, member.release_a[0]),
            (4, member.release_a[1]),
            (5, member.release_a[2]),
            (9, member.release_b[0]),
            (10, member.release_b[1]),
            (11, member.release_b[2]),
        ]
        .into_iter()
        .filter_map(|(dof, on)| on.then_some(dof))
        .collect();
        let kept: Vec<usize> = (0..12).filter(|d| !released.contains(d)).collect();
        // Static condensation of released rotational DOFs: stiffness part.
        let k_kk = submatrix(&k_local, &kept, &kept);
        let (k_eff, k_rk, k_rr) = if released.is_empty() {
            (k_kk, DMatrix::zeros(0, kept.len()), None)
        } else {
            let k_rr = submatrix(&k_local, &released, &released);
            let k_rk = submatrix(&k_local, &released, &kept);
            let chol = k_rr.clone().cholesky().ok_or_else(|| {
                invalid("End releases leave the member with a free rigid rotation")
            })?;
            // Cholesky may formally factor a semidefinite block (e.g. torsion
            // released at both ends); refuse near-zero pivots explicitly.
            let pivot_scale = (0..k_rr.nrows())
                .map(|i| k_rr[(i, i)].abs())
                .fold(0., f64::max);
            if chol
                .l_dirty()
                .diagonal()
                .iter()
                .any(|&v| !v.is_finite() || v * v <= MIN_NORMALIZED_PIVOT * pivot_scale)
            {
                return Err(invalid(
                    "End releases leave the member with a free rigid rotation",
                ));
            }
            let k_eff = &k_kk - k_rk.transpose() * chol.solve(&k_rk);
            if k_eff.iter().any(|v| !v.is_finite()) {
                return Err(numeric());
            }
            (k_eff, k_rk, Some(chol))
        };
        beams.push(Beam {
            nodes: [a, b],
            length,
            rotation,
            k_local,
            k_eff,
            kept,
            released,
            k_rk,
            k_rr,
        });
    }
    Ok(beams)
}

/// Validate one load case against the structure and convert member loads to
/// work-equivalent nodal loads in local axes.
pub(super) fn prepare_loads(beams: &[Beam], load_set: &LoadSet, n: usize) -> Result<Vec<BeamForces>> {
    if load_set.forces_n.len() != n
        || load_set.moments_nmm.len() != n
        || load_set.loads.len() > MAX_MEMBER_LOADS
    {
        return Err(invalid(
            "Each node requires a force and a moment; member loads are capped at 512",
        ));
    }
    if load_set
        .forces_n
        .iter()
        .chain(&load_set.moments_nmm)
        .any(|v| !finite3(v))
    {
        return Err(invalid("Nodal loads must be finite"));
    }
    let mut forces: Vec<BeamForces> = beams
        .iter()
        .map(|_| BeamForces {
            loads: Vec::new(),
            f_eq: DVector::zeros(12),
            f_eff: DVector::zeros(0),
        })
        .collect();
    for load in &load_set.loads {
        let (member, local_axes) = match load {
            MemberLoad::PointForce {
                member,
                local_axes,
                ..
            }
            | MemberLoad::PointMoment {
                member,
                local_axes,
                ..
            }
            | MemberLoad::Uniform {
                member,
                local_axes,
                ..
            }
            | MemberLoad::Trapezoidal {
                member,
                local_axes,
                ..
            } => (*member, *local_axes),
        };
        if member >= beams.len() {
            return Err(invalid("Member load references an unknown member"));
        }
        let beam = &beams[member];
        let to_local = |v: [f64; 3]| -> Result<Vector3<f64>> {
            if !finite3(&v) {
                return Err(invalid("Member load vectors must be finite"));
            }
            let v = Vector3::from(v);
            Ok(if local_axes {
                v
            } else {
                beam.rotation.transpose() * v
            })
        };
        let at_checked = |at: f64| -> Result<f64> {
            if !at.is_finite() || at < 0. || at > beam.length {
                return Err(invalid("Point load position must lie on the member"));
            }
            Ok(at)
        };
        let local = match load {
            MemberLoad::PointForce { at_mm, force_n, .. } => LocalLoad::PointForce {
                at_mm: at_checked(*at_mm)?,
                force: to_local(*force_n)?,
            },
            MemberLoad::PointMoment {
                at_mm,
                moment_nmm,
                ..
            } => LocalLoad::PointMoment {
                at_mm: at_checked(*at_mm)?,
                moment: to_local(*moment_nmm)?,
            },
            MemberLoad::Uniform { force_n_per_mm, .. } => LocalLoad::Uniform {
                w: to_local(*force_n_per_mm)?,
            },
            MemberLoad::Trapezoidal {
                from_n_per_mm,
                to_n_per_mm,
                ..
            } => LocalLoad::Trapezoidal {
                w_a: to_local(*from_n_per_mm)?,
                w_b: to_local(*to_n_per_mm)?,
            },
        };
        forces[member].loads.push(local);
    }
    for (beam, bf) in beams.iter().zip(&mut forces) {
        let mut f_eq = DVector::<f64>::zeros(12);
        for load in &bf.loads {
            equivalent_load(load, beam.length, &mut f_eq);
        }
        if f_eq.iter().any(|v| !v.is_finite()) {
            return Err(numeric());
        }
        // Condensation of the equivalent load vector (stiffness part is
        // already factored in the load-independent Beam).
        let f_k = subvector(&f_eq, &beam.kept);
        bf.f_eff = if beam.released.is_empty() {
            f_k
        } else {
            let chol = beam.k_rr.as_ref().expect("validated condensation");
            let f_r = subvector(&f_eq, &beam.released);
            let f_eff = f_k - beam.k_rk.transpose() * chol.solve(&f_r);
            if f_eff.iter().any(|v| !v.is_finite()) {
                return Err(numeric());
            }
            f_eff
        };
        bf.f_eq = f_eq;
    }
    Ok(forces)
}

/// Section resultants at one station: force/moment exerted by the downstream
/// part of the member on the upstream part, in local axes. Loads exactly at
/// the station count as upstream (values are right-side limits).
pub(super) fn station_resultants(beam: &Beam, forces: &BeamForces, end_on_element: &[f64; 6], x: f64) -> Station {
    let mut force = Vector3::new(end_on_element[0], end_on_element[1], end_on_element[2]);
    let mut moment = Vector3::new(end_on_element[3], end_on_element[4], end_on_element[5]);
    // Moment of the end force about the cut point (node A sits at local origin).
    moment += Vector3::new(-x, 0., 0.).cross(&force);
    for load in &forces.loads {
        match load {
            LocalLoad::PointForce { at_mm, force: p } => {
                if *at_mm <= x {
                    force += p;
                    moment += Vector3::new(at_mm - x, 0., 0.).cross(p);
                }
            }
            LocalLoad::PointMoment { at_mm, moment: m } => {
                if *at_mm <= x {
                    moment += m;
                }
            }
            LocalLoad::Uniform { w } => {
                let resultant = *w * x;
                force += resultant;
                moment += Vector3::new(x / 2. - x, 0., 0.).cross(&resultant);
            }
            LocalLoad::Trapezoidal { w_a, w_b } => {
                let slope = (*w_b - *w_a) / beam.length;
                let resultant = *w_a * x + slope * (x * x / 2.);
                let first_moment = *w_a * (x * x / 2.) + slope * (x * x * x / 3.);
                let scale = 1e-12 * (1. + w_a.norm() * beam.length);
                let centroid = if resultant.norm() > scale {
                    first_moment.dot(&resultant) / resultant.dot(&resultant)
                } else {
                    x / 2.
                };
                force += resultant;
                moment += Vector3::new(centroid - x, 0., 0.).cross(&resultant);
            }
        }
    }
    let s = -force;
    let m = -moment;
    Station {
        x_mm: x,
        axial_n: s.x,
        shear_y_n: s.y,
        shear_z_n: s.z,
        torsion_nmm: m.x,
        moment_y_nmm: m.y,
        moment_z_nmm: m.z,
    }
}

/// Validate the structure and assemble the global stiffness; no factorization.
/// `springs` adds linear spring stiffness to global DOF diagonals; `restrained`
/// is the effective mask (rigid restraints plus active unilateral bounds).
pub(super) fn assemble_stiffness(
    nodes_mm: &[[f64; 3]],
    members: &[Member],
    restrained: &[[bool; 6]],
    springs: &[(usize, f64)],
) -> Result<(Vec<Beam>, DMatrix<f64>)> {
    let beams = validate_structure(nodes_mm, members, restrained)?;
    let n = nodes_mm.len();
    let ndof = n * 6;
    let global_dof = |beam: &Beam, local: usize| beam.nodes[local / 6] * 6 + local % 6;
    let mut stiffness = DMatrix::<f64>::zeros(ndof, ndof);
    for beam in &beams {
        let t = transformation(&beam.rotation);
        // Scatter the condensed local system into a full 12-system, then rotate.
        let mut k_full = DMatrix::<f64>::zeros(12, 12);
        for (i, &di) in beam.kept.iter().enumerate() {
            for (j, &dj) in beam.kept.iter().enumerate() {
                k_full[(di, dj)] = beam.k_eff[(i, j)];
            }
        }
        let k_glob = &t * &k_full * t.transpose();
        if k_glob.iter().any(|v| !v.is_finite()) {
            return Err(numeric());
        }
        for d1 in 0..12 {
            let g1 = global_dof(beam, d1);
            for d2 in 0..12 {
                stiffness[(g1, global_dof(beam, d2))] += k_glob[(d1, d2)];
            }
        }
    }
    for &(dof, k) in springs {
        stiffness[(dof, dof)] += k;
    }
    if stiffness.iter().any(|v| !v.is_finite()) {
        return Err(numeric());
    }
    Ok((beams, stiffness))
}

/// Assemble the global stiffness and factor it once. The factorization is
/// shared by every load case of `solve_envelopes`. A singular refusal names
/// the implicated DOFs; the diagnosing LDLᵀ runs on the error path only.
pub(super) fn assemble(
    nodes_mm: &[[f64; 3]],
    members: &[Member],
    restrained: &[[bool; 6]],
    springs: &[(usize, f64)],
) -> Result<Assembled> {
    let (beams, stiffness) = assemble_stiffness(nodes_mm, members, restrained, springs)?;
    let n = nodes_mm.len();
    let ndof = n * 6;

    // Equilibrated Cholesky on free DOFs, mirroring the truss solver.
    let restraint = |dof: usize| restrained[dof / 6][dof % 6];
    let free: Vec<usize> = (0..ndof).filter(|&i| !restraint(i)).collect();
    let mut scales = Vec::new();
    let mut factor = None;
    if !free.is_empty() {
        scales.reserve(free.len());
        let mut zero_diagonal = false;
        for &i in &free {
            let diagonal = stiffness[(i, i)];
            if diagonal <= 0. {
                zero_diagonal = true;
                break;
            }
            scales.push(diagonal.sqrt());
        }
        if zero_diagonal {
            return Err(singular_diagnosed(&stiffness, &free));
        }
        let reduced = DMatrix::from_fn(free.len(), free.len(), |i, j| {
            stiffness[(free[i], free[j])] / scales[i] / scales[j]
        });
        if reduced.iter().any(|v| !v.is_finite()) {
            return Err(numeric());
        }
        let Some(chol) = reduced.cholesky() else {
            return Err(singular_diagnosed(&stiffness, &free));
        };
        if chol
            .l_dirty()
            .diagonal()
            .iter()
            .any(|&v| !v.is_finite() || v * v <= MIN_NORMALIZED_PIVOT)
        {
            return Err(singular_diagnosed(&stiffness, &free));
        }
        factor = Some(chol);
    }
    let matrix_norm = (0..ndof)
        .map(|i| (0..ndof).map(|j| stiffness[(i, j)].abs()).sum::<f64>())
        .fold(0., f64::max);
    if !matrix_norm.is_finite() {
        return Err(numeric());
    }
    Ok(Assembled {
        beams,
        n,
        restrained: restrained.to_vec(),
        stiffness,
        free,
        scales,
        factor,
        matrix_norm,
    })
}

/// Explain why a structure would be refused as singular, or confirm it is
/// stable. Load-independent: the restraint mask, supports, and end releases
/// decide. Springs and unilateral supports are all treated as engaged,
/// matching the first active-set iteration of `solve`.
pub fn diagnose(
    nodes_mm: &[[f64; 3]],
    members: &[Member],
    restrained: &[[bool; 6]],
    supports: &[Support],
) -> Result<SingularityDiagnosis> {
    let n = nodes_mm.len();
    validate_supports(supports, n, restrained)?;
    let mut mask = restrained.to_vec();
    let mut diagonal = Vec::new();
    for support in supports {
        let (node, dof) = support.node_dof();
        match support {
            Support::Spring { stiffness, .. }
            | Support::LowerSpring { stiffness, .. }
            | Support::UpperSpring { stiffness, .. } => {
                diagonal.push((node * 6 + dof, *stiffness));
            }
            Support::LowerBound { .. } | Support::UpperBound { .. } => mask[node][dof] = true,
        }
    }
    let (_beams, stiffness) = assemble_stiffness(nodes_mm, members, &mask, &diagonal)?;
    let free: Vec<usize> = (0..n * 6).filter(|&i| !mask[i / 6][i % 6]).collect();
    Ok(diagnose_singular(&stiffness, &free, &DOF_NAMES))
}
