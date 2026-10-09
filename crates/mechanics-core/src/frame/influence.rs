use super::*;

/// Member resultant kind for an influence line target, local axes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Resultant {
    Axial,
    ShearY,
    ShearZ,
    Torsion,
    MomentY,
    MomentZ,
}

/// Response quantity whose influence line is requested.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum InfluenceTarget {
    /// Nodal translation (dof 0-2) or rotation (dof 3-5).
    Displacement { node: usize, dof: usize },
    /// Support reaction component; the DOF must be restrained.
    Reaction { node: usize, dof: usize },
    /// Member resultant at `at_mm` from node A, local axes.
    MemberResultant {
        member: usize,
        at_mm: f64,
        resultant: Resultant,
    },
}

/// Influence line values, one per requested load position.
#[derive(Clone, Debug)]
pub struct InfluenceResponse {
    /// Target response under the moving force at each position.
    pub values: Vec<f64>,
}

/// Influence line of one response quantity: the structure is solved exactly
/// for the moving force vector `force_n` placed at every `positions` entry
/// (member, distance from node A), one independent solve per position — no
/// reciprocity shortcuts, so springs and end releases behave identically to
/// `solve`. Member resultants are evaluated exactly at the requested section
/// (not interpolated from the diagram grid). Supports must be rigid or
/// bidirectional springs — unilateral contacts make the response nonlinear.
pub fn influence(
    nodes_mm: &[[f64; 3]],
    members: &[Member],
    restrained: &[[bool; 6]],
    supports: &[Support],
    positions: &[(usize, f64)],
    force_n: [f64; 3],
    target: &InfluenceTarget,
) -> Result<InfluenceResponse> {
    if positions.is_empty() || positions.len() > MAX_INFLUENCE_POSITIONS {
        return Err(invalid("Influence lines admit 1-1024 load positions"));
    }
    if !finite3(&force_n) || force_n.iter().all(|&v| v == 0.) {
        return Err(invalid("The moving force vector must be finite and nonzero"));
    }
    validate_supports(supports, nodes_mm.len(), restrained)?;
    if supports.iter().any(|s| s.is_unilateral()) {
        return Err(invalid(
            "Influence lines admit rigid restraints and bidirectional springs only",
        ));
    }
    let springs: Vec<(usize, f64)> = supports
        .iter()
        .filter_map(|s| match s {
            Support::Spring {
                node,
                dof,
                stiffness,
            } => Some((node * 6 + dof, *stiffness)),
            _ => None,
        })
        .collect();
    let assembled = assemble(nodes_mm, members, restrained, &springs)?;
    let lengths: Vec<f64> = assembled.beams.iter().map(|b| b.length).collect();
    let on_member = |m: usize, at: f64| {
        m < members.len() && at.is_finite() && at >= 0. && at <= lengths[m] * (1. + 1e-9)
    };
    for &(m, at) in positions {
        if !on_member(m, at) {
            return Err(invalid("Load positions must lie on their member"));
        }
    }
    match *target {
        InfluenceTarget::Displacement { node, dof } => {
            if node >= nodes_mm.len() || dof >= 6 {
                return Err(invalid("Displacement target needs node < nodes, dof < 6"));
            }
        }
        InfluenceTarget::Reaction { node, dof } => {
            if node >= nodes_mm.len() || dof >= 6 {
                return Err(invalid("Reaction target needs node < nodes, dof < 6"));
            }
            if !restrained[node][dof] {
                return Err(invalid("Reaction influence requires a restrained DOF"));
            }
        }
        InfluenceTarget::MemberResultant {
            member, at_mm, ..
        } => {
            if !on_member(member, at_mm) {
                return Err(invalid("The target section must lie on its member"));
            }
        }
    }
    let mut values = Vec::with_capacity(positions.len());
    for &(m, at) in positions {
        let at = at.min(lengths[m]);
        let load_set = LoadSet {
            forces_n: vec![[0.; 3]; nodes_mm.len()],
            moments_nmm: vec![[0.; 3]; nodes_mm.len()],
            loads: vec![MemberLoad::PointForce {
                member: m,
                at_mm: at,
                force_n,
                local_axes: false,
            }],
        };
        let response = solve_case(&assembled, &load_set)?;
        let value = match *target {
            InfluenceTarget::Displacement { node, dof } => {
                if dof < 3 {
                    response.displacements_mm[node][dof]
                } else {
                    response.rotations_rad[node][dof - 3]
                }
            }
            InfluenceTarget::Reaction { node, dof } => {
                if dof < 3 {
                    response.reactions_n[node][dof]
                } else {
                    response.reaction_moments_nmm[node][dof - 3]
                }
            }
            InfluenceTarget::MemberResultant {
                member,
                at_mm,
                resultant,
            } => {
                // Recover the target member's end forces and evaluate the
                // resultant exactly at the requested section.
                let forces = prepare_loads(&assembled.beams, &load_set, assembled.n)?;
                let beam = &assembled.beams[member];
                let mut displacement = DVector::<f64>::zeros(assembled.n * 6);
                for i in 0..assembled.n {
                    for c in 0..3 {
                        displacement[i * 6 + c] = response.displacements_mm[i][c];
                        displacement[i * 6 + 3 + c] = response.rotations_rad[i][c];
                    }
                }
                let t = transformation(&beam.rotation);
                let global_dof = |local: usize| beam.nodes[local / 6] * 6 + local % 6;
                let mut u_glob = DVector::<f64>::zeros(12);
                for d in 0..12 {
                    u_glob[d] = displacement[global_dof(d)];
                }
                let mut u_local = t.transpose() * u_glob;
                if !beam.released.is_empty() {
                    let chol = beam.k_rr.as_ref().expect("validated condensation");
                    let u_k = subvector(&u_local, &beam.kept);
                    let f_r = subvector(&forces[member].f_eq, &beam.released);
                    let u_r = chol.solve(&(f_r - &beam.k_rk * u_k));
                    for (i, &d) in beam.released.iter().enumerate() {
                        u_local[d] = u_r[i];
                    }
                }
                let q = &beam.k_local * &u_local - &forces[member].f_eq;
                let mut end_on_element = [0.; 6];
                end_on_element.copy_from_slice(&q.as_slice()[..6]);
                let s = station_resultants(beam, &forces[member], &end_on_element, at_mm.min(lengths[member]));
                match resultant {
                    Resultant::Axial => s.axial_n,
                    Resultant::ShearY => s.shear_y_n,
                    Resultant::ShearZ => s.shear_z_n,
                    Resultant::Torsion => s.torsion_nmm,
                    Resultant::MomentY => s.moment_y_nmm,
                    Resultant::MomentZ => s.moment_z_nmm,
                }
            }
        };
        if !value.is_finite() {
            return Err(numeric());
        }
        values.push(value);
    }
    Ok(InfluenceResponse { values })
}
