use super::*;

/// Simple (Euler–Bernoulli-form) geometric stiffness of one member under
/// axial force `n` (tension positive): bending DOFs only, no torsion coupling.
/// With shear areas Kg keeps this EB form, matching the equivalent-load
/// approximation. Axial member loads vary N along a member; split such members
/// so the constant-N assumption holds per element.
pub(super) fn beam_geometric_stiffness(n: f64, l: f64) -> DMatrix<f64> {
    let mut kg = DMatrix::<f64>::zeros(12, 12);
    let c = n / (30. * l);
    // Bending about local z, DOFs [v1, rz1, v2, rz2].
    let dz = [1usize, 5, 7, 11];
    let bz = [
        [36., 3. * l, -36., 3. * l],
        [3. * l, 4. * l * l, -3. * l, -l * l],
        [-36., -3. * l, 36., -3. * l],
        [3. * l, -l * l, -3. * l, 4. * l * l],
    ];
    // Bending about local y, DOFs [w1, ry1, w2, ry2]; θy = −dw/dx flips the
    // L-odd couplings, mirroring the elastic block.
    let dy = [2usize, 4, 8, 10];
    let by = [
        [36., -3. * l, -36., -3. * l],
        [-3. * l, 4. * l * l, 3. * l, -l * l],
        [-36., 3. * l, 36., 3. * l],
        [-3. * l, -l * l, 3. * l, 4. * l * l],
    ];
    for (dofs, block) in [(dz, bz), (dy, by)] {
        for i in 0..4 {
            for j in 0..4 {
                kg[(dofs[i], dofs[j])] = c * block[i][j];
            }
        }
    }
    kg
}

/// Restrict a full 12×12 local matrix to the kept DOFs through the elastic
/// condensation subspace u = T·u_k, T = [I; −K_rr⁻¹·K_rk] — exact for the
/// stiffness itself, a consistent subspace restriction for Kg under releases.
pub(super) fn condense_local(beam: &Beam, full: &DMatrix<f64>) -> DMatrix<f64> {
    if beam.released.is_empty() {
        return submatrix(full, &beam.kept, &beam.kept);
    }
    let mut t = DMatrix::<f64>::zeros(12, beam.kept.len());
    for (col, &d) in beam.kept.iter().enumerate() {
        t[(d, col)] = 1.;
    }
    let m = -beam.k_rr.as_ref().unwrap().solve(&beam.k_rk);
    for (row, &d) in beam.released.iter().enumerate() {
        for col in 0..beam.kept.len() {
            t[(d, col)] = m[(row, col)];
        }
    }
    t.transpose() * full * &t
}

/// Global geometric stiffness from per-member reference axial forces.
pub(super) fn assemble_geometric(beams: &[Beam], axial: &[f64], n: usize) -> Result<DMatrix<f64>> {
    let mats: Vec<DMatrix<f64>> = beams
        .iter()
        .zip(axial)
        .map(|(beam, &axial_n)| beam_geometric_stiffness(axial_n, beam.length))
        .collect();
    assemble_member_matrices(beams, &mats, n)
}

/// Shared condense–scatter–rotate assembly of symmetric 12×12 local member
/// matrices (geometric stiffness, mass).
pub(super) fn assemble_member_matrices(beams: &[Beam], mats: &[DMatrix<f64>], n: usize) -> Result<DMatrix<f64>> {
    let ndof = n * 6;
    let global_dof = |beam: &Beam, local: usize| beam.nodes[local / 6] * 6 + local % 6;
    let mut global = DMatrix::<f64>::zeros(ndof, ndof);
    for (beam, local) in beams.iter().zip(mats) {
        let effective = condense_local(beam, local);
        if effective.iter().any(|v| !v.is_finite()) {
            return Err(numeric());
        }
        let mut full = DMatrix::<f64>::zeros(12, 12);
        for (i, &di) in beam.kept.iter().enumerate() {
            for (j, &dj) in beam.kept.iter().enumerate() {
                full[(di, dj)] = effective[(i, j)];
            }
        }
        let t = transformation(&beam.rotation);
        let rotated = &t * &full * t.transpose();
        if rotated.iter().any(|v| !v.is_finite()) {
            return Err(numeric());
        }
        for d1 in 0..12 {
            let g1 = global_dof(beam, d1);
            for d2 in 0..12 {
                global[(g1, global_dof(beam, d2))] += rotated[(d1, d2)];
            }
        }
    }
    if global.iter().any(|v| !v.is_finite()) {
        return Err(numeric());
    }
    Ok(global)
}

/// One buckling mode of the reference load state, nodal form.
#[derive(Clone, Debug)]
pub struct FrameBucklingMode {
    /// Signed factor on the reference loads (buckling load = λ·reference);
    /// negative means the structure buckles under the reversed load.
    pub load_factor: f64,
    /// Mode shape over all DOFs, max |component| = 1; restrained DOFs are zero.
    pub displacements: Vec<[f64; 3]>,
    pub rotations: Vec<[f64; 3]>,
    pub relative_residual: f64,
}

/// Eigenvalue buckling of the reference load state.
#[derive(Clone, Debug)]
pub struct FrameBucklingResponse {
    /// Ascending |load_factor|; the first entry is the critical mode.
    pub modes: Vec<FrameBucklingMode>,
    /// Member axial forces of the reference state (tension positive) that the
    /// geometric stiffness is built from.
    pub axial_forces_n: Vec<f64>,
    pub free_dofs: usize,
}

/// Linear buckling of a frame under a reference load set. The elastic
/// stiffness is factored once (same path as `solve`); the geometric stiffness
/// uses each member's reference axial force. Supports must be rigid or
/// bidirectional springs — unilateral contacts make the reference state
/// nonlinear, so they are refused. Not a certified stability calculation.
pub fn buckling(
    nodes_mm: &[[f64; 3]],
    members: &[Member],
    restrained: &[[bool; 6]],
    supports: &[Support],
    reference: &LoadSet,
    modes: usize,
) -> Result<FrameBucklingResponse> {
    if modes == 0 || modes > MAX_MODES {
        return Err(invalid("Buckling returns between 1 and 8 modes"));
    }
    validate_supports(supports, nodes_mm.len(), restrained)?;
    if supports.iter().any(|s| s.is_unilateral()) {
        return Err(invalid(
            "Buckling admits rigid restraints and bidirectional springs only",
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
    let reference_response = solve_case(&assembled, reference)?;
    let axial: Vec<f64> = reference_response
        .members
        .iter()
        .map(|m| m.stations[0].axial_n)
        .collect();
    let geometric = assemble_geometric(&assembled.beams, &axial, assembled.n)?;
    let solved = match &assembled.factor {
        Some(factor) => solve_modes(
            factor,
            &assembled.scales,
            &assembled.free,
            &assembled.stiffness,
            &geometric,
            modes,
        )?,
        None => Vec::new(),
    };
    let modes_out = solved
        .into_iter()
        .map(|mode| FrameBucklingMode {
            load_factor: mode.load_factor,
            displacements: (0..assembled.n)
                .map(|i| std::array::from_fn(|k| mode.shape[i * 6 + k]))
                .collect(),
            rotations: (0..assembled.n)
                .map(|i| std::array::from_fn(|k| mode.shape[i * 6 + 3 + k]))
                .collect(),
            relative_residual: mode.relative_residual,
        })
        .collect();
    Ok(FrameBucklingResponse {
        modes: modes_out,
        axial_forces_n: axial,
        free_dofs: assembled.free.len(),
    })
}

/// Consistent (Euler–Bernoulli) mass matrix of one member, local axes.
/// Density in t/mm³ keeps the mm/N units consistent (1 t·mm/s² = 1 N), so
/// frequencies come out in Hz. Torsion uses the polar moment ρ(Iyy+Izz).
pub(super) fn beam_mass_consistent(rho: f64, area: f64, iyy: f64, izz: f64, l: f64) -> DMatrix<f64> {
    let mut m = DMatrix::<f64>::zeros(12, 12);
    let axial = rho * area * l / 6.;
    let torsion = rho * (iyy + izz) * l / 6.;
    for (dofs, value) in [([0usize, 6], axial), ([3usize, 9], torsion)] {
        for d in dofs {
            for e in dofs {
                m[(d, e)] = value * if d == e { 2. } else { 1. };
            }
        }
    }
    let c = rho * area * l / 420.;
    // Bending about local z, DOFs [v1, rz1, v2, rz2].
    let dz = [1usize, 5, 7, 11];
    let bz = [
        [156., 22. * l, 54., -13. * l],
        [22. * l, 4. * l * l, 13. * l, -3. * l * l],
        [54., 13. * l, 156., -22. * l],
        [-13. * l, -3. * l * l, -22. * l, 4. * l * l],
    ];
    // Bending about local y, DOFs [w1, ry1, w2, ry2] (θy = −dw/dx pattern).
    let dy = [2usize, 4, 8, 10];
    let by = [
        [156., -22. * l, 54., 13. * l],
        [-22. * l, 4. * l * l, -13. * l, -3. * l * l],
        [54., -13. * l, 156., 22. * l],
        [13. * l, -3. * l * l, 22. * l, 4. * l * l],
    ];
    for (dofs, block) in [(dz, bz), (dy, by)] {
        for i in 0..4 {
            for j in 0..4 {
                m[(dofs[i], dofs[j])] = c * block[i][j];
            }
        }
    }
    m
}

/// Lumped mass: half the member mass per end, translational DOFs only.
/// Rotational DOFs stay massless and drop out of the spectrum.
pub(super) fn beam_mass_lumped(rho: f64, area: f64, l: f64) -> DMatrix<f64> {
    let mut m = DMatrix::<f64>::zeros(12, 12);
    let half = rho * area * l / 2.;
    for base in [0usize, 6] {
        for axis in 0..3 {
            m[(base + axis, base + axis)] = half;
        }
    }
    m
}

/// One vibration mode, nodal form.
#[derive(Clone, Debug)]
pub struct FrameModalMode {
    pub frequency_hz: f64,
    /// Angular frequency ω = 2πf.
    pub omega_rad_s: f64,
    /// Mode shape over all DOFs, max |component| = 1; restrained DOFs are zero.
    pub displacements: Vec<[f64; 3]>,
    pub rotations: Vec<[f64; 3]>,
    /// ‖Kφ − ω²Mφ‖∞ / (‖Kφ‖∞ + ‖ω²Mφ‖∞) on the free DOFs.
    pub relative_residual: f64,
}

/// Natural frequencies and mode shapes of the structure.
#[derive(Clone, Debug)]
pub struct FrameModalResponse {
    /// Ascending frequency.
    pub modes: Vec<FrameModalMode>,
    /// Total member mass in tonnes (1 t·mm/s² = 1 N with mm units).
    pub total_mass_t: f64,
    pub free_dofs: usize,
}

/// Small-displacement modal analysis: Kφ = ω²Mφ with the member mass from
/// `densities_t_mm3` (t/mm³; steel ≈ 7.85e-9; zero means a massless member).
/// Supports must be rigid or bidirectional springs — unilateral contacts make
/// the vibration state nonlinear, so they are refused. Massless free DOFs have
/// no finite frequency and do not appear among the modes. Not a certified
/// dynamic calculation.
pub fn modal(
    nodes_mm: &[[f64; 3]],
    members: &[Member],
    restrained: &[[bool; 6]],
    supports: &[Support],
    densities_t_mm3: &[f64],
    mass_model: MassModel,
    modes: usize,
) -> Result<FrameModalResponse> {
    if modes == 0 || modes > MAX_MODES {
        return Err(invalid("Modal analysis returns between 1 and 8 modes"));
    }
    if densities_t_mm3.len() != members.len() {
        return Err(invalid("Each member requires a density (0 for massless)"));
    }
    if densities_t_mm3.iter().any(|d| !d.is_finite() || *d < 0.) {
        return Err(invalid("Densities must be finite and nonnegative"));
    }
    validate_supports(supports, nodes_mm.len(), restrained)?;
    if supports.iter().any(|s| s.is_unilateral()) {
        return Err(invalid(
            "Modal analysis admits rigid restraints and bidirectional springs only",
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
    let mats: Vec<DMatrix<f64>> = assembled
        .beams
        .iter()
        .zip(members)
        .zip(densities_t_mm3)
        .map(|((beam, member), &rho)| match mass_model {
            MassModel::Lumped => beam_mass_lumped(rho, member.area_mm2, beam.length),
            MassModel::Consistent => beam_mass_consistent(
                rho,
                member.area_mm2,
                member.iyy_mm4,
                member.izz_mm4,
                beam.length,
            ),
        })
        .collect();
    let mass = assemble_member_matrices(&assembled.beams, &mats, assembled.n)?;
    let total_mass_t: f64 = assembled
        .beams
        .iter()
        .zip(members)
        .zip(densities_t_mm3)
        .map(|((beam, member), &rho)| rho * member.area_mm2 * beam.length)
        .sum();
    // Kφ = ω²Mφ is the shared pencil with geometric := −M; load_factor = ω².
    let negative_mass = -&mass;
    let solved = match &assembled.factor {
        Some(factor) => solve_modes(
            factor,
            &assembled.scales,
            &assembled.free,
            &assembled.stiffness,
            &negative_mass,
            modes,
        )?,
        None => Vec::new(),
    };
    let modes_out = solved
        .into_iter()
        .map(|mode| {
            let omega = mode.load_factor.sqrt();
            FrameModalMode {
                frequency_hz: omega / (2. * std::f64::consts::PI),
                omega_rad_s: omega,
                displacements: (0..assembled.n)
                    .map(|i| std::array::from_fn(|k| mode.shape[i * 6 + k]))
                    .collect(),
                rotations: (0..assembled.n)
                    .map(|i| std::array::from_fn(|k| mode.shape[i * 6 + 3 + k]))
                    .collect(),
                relative_residual: mode.relative_residual,
            }
        })
        .collect();
    Ok(FrameModalResponse {
        modes: modes_out,
        total_mass_t,
        free_dofs: assembled.free.len(),
    })
}
