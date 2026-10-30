//! Strength-of-materials estimates from already-cut contours.
//!
//! Section estimates: area, centroid, second moments, section moduli,
//! a scanline wall-width probe, and σ = Mc/I (+ N/A). Not a surface kernel,
//! not a subdivision cage, not gear/thread generation (those live in RushGraph).
//! The separate `truss` module solves bounded linear pin-jointed bar systems.
//! Neither module is a print process or a material certificate. The host (CAD)
//! sections a mesh; this crate never holds CAD handles. Coordinates are
//! millimeters, force is newtons, stress is MPa (N/mm²).
#![feature(
    try_blocks,
    gen_blocks,
    yield_expr,
    super_let,
    deref_patterns,
    yeet_expr
)]
#![allow(unused_features)]

pub const MAX_POINTS: usize = 16_384;

pub mod bonded_solid;
pub mod buckling;
pub mod combos;
pub mod diagnostics;
pub mod frame;
pub mod print_strength;
pub mod section;
pub mod structural_profiles;
pub mod thermal_strength;
pub mod truss;
pub mod truss_loads;
pub mod truss_topology;

pub use math_core::{Error, Result};
use math_core::{cross2, sub2};
pub use planar_geometry::{LayerSection, MAX_LAYERS};

/// How member mass is distributed in modal analysis.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MassModel {
    /// Half of each member's mass goes to each end's translational DOFs.
    Lumped,
    /// Full shape-function mass coupling (rotational inertia included for
    /// frame members; isotropic ρAL/6·[[2I,I],[I,2I]] for truss bars).
    Consistent,
}

/// Bending moments about the section x/y axes and optional axial force.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct LoadCase {
    pub moment_x_nmm: f64,
    pub moment_y_nmm: f64,
    pub axial_n: f64,
    /// Engineering allowable. Not a certified material property.
    pub allowable_mpa: f64,
}

#[derive(Clone, Debug, PartialEq)]
pub struct SectionReport {
    pub z_mm: f64,
    pub area_mm2: f64,
    pub centroid: [f64; 2],
    pub ixx_mm4: f64,
    pub iyy_mm4: f64,
    pub ixy_mm4: f64,
    pub wx_mm3: f64,
    pub wy_mm3: f64,
    /// Narrowest scanline chord through material. Zero if the section is empty.
    pub min_wall_mm: f64,
    pub bending_mpa: f64,
    pub axial_mpa: f64,
    pub von_mises_mpa: f64,
    pub factor_of_safety: f64,
}

#[derive(Clone, Debug, PartialEq)]
pub struct StrengthReport {
    pub layers: Vec<SectionReport>,
    pub weakest_index: usize,
}

fn invalid(code: &'static str, message: &str) -> Error {
    Error::new(code, message)
}

fn require_finite(value: f64, message: &str) -> Result<()> {
    if value.is_finite() {
        Ok(())
    } else {
        Err(invalid("MECHANICS_INVALID_INPUT", message))
    }
}

fn require_section(section: &LayerSection) -> Result<()> {
    require_finite(section.z_mm, "Layer height must be finite")?;
    let mut points = 0usize;
    for ring in &section.contours {
        if ring.len() < 3 {
            return Err(invalid(
                "MECHANICS_INVALID_SECTION",
                "Each contour needs at least three points",
            ));
        }
        points = points
            .checked_add(ring.len())
            .ok_or_else(|| invalid("MECHANICS_POINT_LIMIT", "Section exceeds point budget"))?;
        if points > MAX_POINTS {
            return Err(invalid(
                "MECHANICS_POINT_LIMIT",
                "Section exceeds 16384 points",
            ));
        }
        for point in ring {
            require_finite(point[0], "Contour coordinates must be finite")?;
            require_finite(point[1], "Contour coordinates must be finite")?;
        }
    }
    Ok(())
}

fn require_load(load: &LoadCase) -> Result<()> {
    require_finite(load.moment_x_nmm, "Moment must be finite")?;
    require_finite(load.moment_y_nmm, "Moment must be finite")?;
    require_finite(load.axial_n, "Axial force must be finite")?;
    if !load.allowable_mpa.is_finite() || load.allowable_mpa <= 0.0 {
        return Err(invalid(
            "MECHANICS_INVALID_MATERIAL",
            "Allowable stress must be finite and positive",
        ));
    }
    Ok(())
}

fn polygon_integrals(ring: &[[f64; 2]]) -> (f64, f64, f64, f64, f64, f64) {
    let mut twice_area = 0.0;
    let mut cx = 0.0;
    let mut cy = 0.0;
    let mut ixx = 0.0;
    let mut iyy = 0.0;
    let mut ixy = 0.0;
    for i in 0..ring.len() {
        let a = ring[i];
        let b = ring[(i + 1) % ring.len()];
        let cross = a[0] * b[1] - b[0] * a[1];
        twice_area += cross;
        cx += (a[0] + b[0]) * cross;
        cy += (a[1] + b[1]) * cross;
        ixx += (a[1] * a[1] + a[1] * b[1] + b[1] * b[1]) * cross;
        iyy += (a[0] * a[0] + a[0] * b[0] + b[0] * b[0]) * cross;
        ixy += (a[0] * b[1] + 2.0 * a[0] * a[1] + 2.0 * b[0] * b[1] + b[0] * a[1]) * cross;
    }
    (twice_area, cx, cy, ixx, iyy, ixy)
}

fn extrema(section: &LayerSection, centroid: [f64; 2]) -> (f64, f64) {
    let mut max_x = 0.0_f64;
    let mut max_y = 0.0_f64;
    for ring in &section.contours {
        for point in ring {
            max_x = max_x.max((point[0] - centroid[0]).abs());
            max_y = max_y.max((point[1] - centroid[1]).abs());
        }
    }
    (max_x, max_y)
}

fn winding(point: [f64; 2], contours: &[Vec<[f64; 2]>]) -> i32 {
    let mut winding = 0;
    for ring in contours {
        for i in 0..ring.len() {
            let a = ring[i];
            let b = ring[(i + 1) % ring.len()];
            let c = cross2(sub2(b, a), sub2(point, a));
            if a[1] <= point[1] && b[1] > point[1] && c > 0.0 {
                winding += 1;
            }
            if a[1] > point[1] && b[1] <= point[1] && c < 0.0 {
                winding -= 1;
            }
        }
    }
    winding
}





/// Rank already-cut layers. Weakest is the smallest finite factor of safety,
/// then the smallest section modulus.
pub fn analyze_layers(sections: &[LayerSection], load: &LoadCase) -> Result<StrengthReport> {
    require_load(load)?;
    if sections.is_empty() {
        return Err(invalid("MECHANICS_EMPTY_SECTION", "No layers to analyze"));
    }
    if sections.len() > MAX_LAYERS {
        return Err(invalid(
            "MECHANICS_LAYER_LIMIT",
            "Strength plan exceeded 2048 layers",
        ));
    }
    let layers = math_core::par_try_map(sections, 2, |section| analyze_section(section, load))?;
    let weakest_index = layers
        .iter()
        .enumerate()
        .min_by(|(_, a), (_, b)| {
            match (
                a.factor_of_safety.is_finite(),
                b.factor_of_safety.is_finite(),
            ) {
                (true, false) => std::cmp::Ordering::Less,
                (false, true) => std::cmp::Ordering::Greater,
                _ => a
                    .factor_of_safety
                    .total_cmp(&b.factor_of_safety)
                    .then_with(|| a.wx_mm3.min(a.wy_mm3).total_cmp(&b.wx_mm3.min(b.wy_mm3))),
            }
        })
        .map_or(0, |(index, _)| index);
    Ok(StrengthReport {
        layers,
        weakest_index,
    })
}

/// 3D wall thickness analysis result over a triangle mesh.
#[derive(Clone, Debug, PartialEq)]
pub struct WallThickness3dReport {
    pub triangle_thickness_mm: Vec<f64>,
    pub min_thickness_mm: f64,
    pub max_thickness_mm: f64,
    pub thin_triangle_indices: Vec<usize>,
}

/// Casts inward normal rays from each triangle centroid of a closed 3D mesh (`positions` flat xyz,
/// `indices` flat triangle indices) to measure local wall thickness across the part.
pub fn analyze_wall_thickness_3d(
    positions: &[f64],
    indices: &[u32],
    min_threshold_mm: f64,
) -> Result<WallThickness3dReport> {
    if !positions.len().is_multiple_of(3)
        || positions.is_empty()
        || !indices.len().is_multiple_of(3)
        || indices.is_empty()
    {
        return Err(invalid(
            "MECHANICS_INVALID_MESH",
            "Mesh requires non-empty xyz positions and triangle indices",
        ));
    }
    require_finite(min_threshold_mm, "Threshold must be finite")?;
    let v_count = positions.len() / 3;
    let tri_count = indices.len() / 3;
    if tri_count > 50_000 {
        return Err(invalid(
            "MECHANICS_MESH_LIMIT",
            "3D wall thickness analysis supports up to 50000 triangles",
        ));
    }
    let mut tris = Vec::with_capacity(tri_count);
    for t in indices.chunks_exact(3) {
        let (i0, i1, i2) = (t[0] as usize, t[1] as usize, t[2] as usize);
        if i0 >= v_count || i1 >= v_count || i2 >= v_count {
            return Err(invalid(
                "MECHANICS_INVALID_MESH",
                "Triangle index out of bounds",
            ));
        }
        let v0 = [positions[i0 * 3], positions[i0 * 3 + 1], positions[i0 * 3 + 2]];
        let v1 = [positions[i1 * 3], positions[i1 * 3 + 1], positions[i1 * 3 + 2]];
        let v2 = [positions[i2 * 3], positions[i2 * 3 + 1], positions[i2 * 3 + 2]];
        let e1 = [v1[0] - v0[0], v1[1] - v0[1], v1[2] - v0[2]];
        let e2 = [v2[0] - v0[0], v2[1] - v0[1], v2[2] - v0[2]];
        tris.push((v0, e1, e2));
    }
    let thicknesses = math_core::par_map_index(tri_count, 64, |ti| {
        let (v0, e1, e2) = tris[ti];
        let n = [
            e1[1] * e2[2] - e1[2] * e2[1],
            e1[2] * e2[0] - e1[0] * e2[2],
            e1[0] * e2[1] - e1[1] * e2[0],
        ];
        let n_len = (n[0] * n[0] + n[1] * n[1] + n[2] * n[2]).sqrt();
        if n_len <= 1e-12 {
            return 0.0;
        }
        // Inward ray direction is -normal
        let dir = [-n[0] / n_len, -n[1] / n_len, -n[2] / n_len];
        let origin = [
            v0[0] + (e1[0] + e2[0]) / 3.0,
            v0[1] + (e1[1] + e2[1]) / 3.0,
            v0[2] + (e1[2] + e2[2]) / 3.0,
        ];
        let mut best_hit = f64::INFINITY;
        for (oj, &(u0, ue1, ue2)) in tris.iter().enumerate() {
            if ti == oj {
                continue;
            }
            let h = [
                dir[1] * ue2[2] - dir[2] * ue2[1],
                dir[2] * ue2[0] - dir[0] * ue2[2],
                dir[0] * ue2[1] - dir[1] * ue2[0],
            ];
            let a = ue1[0] * h[0] + ue1[1] * h[1] + ue1[2] * h[2];
            if a.abs() <= 1e-12 {
                continue;
            }
            let f = 1.0 / a;
            let s = [origin[0] - u0[0], origin[1] - u0[1], origin[2] - u0[2]];
            let u = f * (s[0] * h[0] + s[1] * h[1] + s[2] * h[2]);
            if !(0.0..=1.0).contains(&u) {
                continue;
            }
            let q = [
                s[1] * ue1[2] - s[2] * ue1[1],
                s[2] * ue1[0] - s[0] * ue1[2],
                s[0] * ue1[1] - s[1] * ue1[0],
            ];
            let v = f * (dir[0] * q[0] + dir[1] * q[1] + dir[2] * q[2]);
            if v < 0.0 || u + v > 1.0 {
                continue;
            }
            let dist = f * (ue2[0] * q[0] + ue2[1] * q[1] + ue2[2] * q[2]);
            if dist > 1e-6 && dist < best_hit {
                best_hit = dist;
            }
        }
        if best_hit.is_finite() { best_hit } else { 0.0 }
    });
    let mut min_t = f64::INFINITY;
    let mut max_t = 0.0_f64;
    let mut thin_indices = Vec::new();
    for (ti, &thick) in thicknesses.iter().enumerate() {
        if thick > 0.0 {
            min_t = min_t.min(thick);
            max_t = max_t.max(thick);
            if thick < min_threshold_mm {
                thin_indices.push(ti);
            }
        }
    }
    Ok(WallThickness3dReport {
        triangle_thickness_mm: thicknesses,
        min_thickness_mm: if min_t.is_finite() { min_t } else { 0.0 },
        max_thickness_mm: max_t,
        thin_triangle_indices: thin_indices,
    })
}

/// Computes Euler column critical buckling load ($P_{\text{cr}} = \pi^2 E I / (K L)^2$) and safety factor
/// for a slender member under compressive axial load `compressive_load_n` (positive in compression).
pub fn euler_buckling_safety_factor(
    young_mpa: f64,
    min_moment_of_inertia_mm4: f64,
    length_mm: f64,
    effective_length_factor_k: f64,
    compressive_load_n: f64,
) -> Result<(f64, f64)> {
    if ![
        young_mpa,
        min_moment_of_inertia_mm4,
        length_mm,
        effective_length_factor_k,
    ]
    .into_iter()
    .all(|v| v.is_finite() && v > 0.0)
        || !compressive_load_n.is_finite()
    {
        return Err(invalid(
            "MECHANICS_INVALID_INPUT",
            "Euler buckling requires positive finite E, I, L, K and finite load",
        ));
    }
    let kl = effective_length_factor_k * length_mm;
    let p_cr = std::f64::consts::PI.powi(2) * young_mpa * min_moment_of_inertia_mm4 / (kl * kl);
    let fos = if compressive_load_n > 0.0 {
        p_cr / compressive_load_n
    } else {
        f64::INFINITY
    };
    Ok((p_cr, fos))
}

/// Estimates fundamental natural frequency $f_1$ (Hz) from static nodal displacements under gravity
/// via the Rayleigh quotient: $\omega_1^2 \approx g \sum m_i u_i / \sum m_i u_i^2$.
pub fn estimate_fundamental_frequency_hz(
    nodal_masses_kg: &[f64],
    static_gravity_deflections_mm: &[f64],
) -> Result<f64> {
    if nodal_masses_kg.is_empty()
        || nodal_masses_kg.len() != static_gravity_deflections_mm.len()
        || nodal_masses_kg.iter().any(|&m| !m.is_finite() || m < 0.0)
        || static_gravity_deflections_mm
            .iter()
            .any(|&u| !u.is_finite() || u < 0.0)
    {
        return Err(invalid(
            "MECHANICS_INVALID_INPUT",
            "Rayleigh frequency estimate requires matching non-negative masses and deflections",
        ));
    }
    let mut num = 0.0;
    let mut den = 0.0;
    for (&m, &u_mm) in nodal_masses_kg.iter().zip(static_gravity_deflections_mm) {
        let u_m = u_mm * 1e-3;
        num += m * u_m;
        den += m * u_m * u_m;
    }
    if den <= 1e-18 || num <= 0.0 {
        return Err(invalid(
            "MECHANICS_INVALID_INPUT",
            "Total mass deflection product must be positive",
        ));
    }
    let g = 9.80665;
    let omega = (g * num / den).sqrt();
    Ok(omega / std::f64::consts::TAU)
}

/// Classical high-cycle fatigue criteria.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FatigueCriterion {
    /// Modified Goodman line: $\sigma_a / S_e + \sigma_m / S_{ut} = 1 / n$
    Goodman,
    /// Soderberg line (conservative, yield-based): $\sigma_a / S_e + \sigma_m / S_y = 1 / n$
    Soderberg,
    /// Gerber parabola: $n \sigma_a / S_e + (n \sigma_m / S_{ut})^2 = 1$
    Gerber,
}

/// Computes high-cycle fatigue factor of safety under fluctuating stress ($\sigma_{\min}$, $\sigma_{\max}$).
pub fn fatigue_safety_factor(
    sigma_min_mpa: f64,
    sigma_max_mpa: f64,
    endurance_limit_mpa: f64,
    yield_strength_mpa: f64,
    ultimate_strength_mpa: f64,
    criterion: FatigueCriterion,
) -> Result<f64> {
    if ![
        sigma_min_mpa,
        sigma_max_mpa,
        endurance_limit_mpa,
        yield_strength_mpa,
        ultimate_strength_mpa,
    ]
    .into_iter()
    .all(f64::is_finite)
        || endurance_limit_mpa <= 0.0
        || yield_strength_mpa <= 0.0
        || ultimate_strength_mpa <= 0.0
        || sigma_max_mpa < sigma_min_mpa
    {
        return Err(invalid(
            "MECHANICS_INVALID_INPUT",
            "Fatigue calculation requires finite stresses with sigma_max >= sigma_min and positive material strengths",
        ));
    }
    let sigma_a = 0.5 * (sigma_max_mpa - sigma_min_mpa);
    let sigma_m = 0.5 * (sigma_max_mpa + sigma_min_mpa).max(0.0);
    if sigma_a <= 1e-12 && sigma_m <= 1e-12 {
        return Ok(f64::INFINITY);
    }
    let fos = match criterion {
        FatigueCriterion::Goodman => {
            1.0 / (sigma_a / endurance_limit_mpa + sigma_m / ultimate_strength_mpa)
        }
        FatigueCriterion::Soderberg => {
            1.0 / (sigma_a / endurance_limit_mpa + sigma_m / yield_strength_mpa)
        }
        FatigueCriterion::Gerber => {
            if sigma_m <= 1e-12 {
                endurance_limit_mpa / sigma_a
            } else {
                let r_a = sigma_a / endurance_limit_mpa;
                let r_m = sigma_m / ultimate_strength_mpa;
                (-r_a + (r_a * r_a + 4.0 * r_m * r_m).sqrt()) / (2.0 * r_m * r_m)
            }
        }
    };
    Ok(fos)
}

#[cfg(test)]
#[path = "tests/lib_tests_integration.rs"]
mod lib_tests;

/// VDI 2230 / ISO 898-1 bolted joint preload, tensile stress area, and clamping safety report.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BoltedJointReport {
    /// ISO 898-1 tensile stress area $A_s = \frac{\pi}{4}(d - 0.9382 P)^2$ in $\text{mm}^2$.
    pub tensile_stress_area_mm2: f64,
    /// Stiffness partition factor $C = k_b / (k_b + k_m)$ (standard steel bolt in steel/alu flange $\approx 0.25$).
    pub stiffness_factor_c: f64,
    /// Total bolt axial tension under external load: $F_b = F_i + C F_{\text{ext}}$ in N.
    pub bolt_axial_force_n: f64,
    /// Remaining clamping force on the joint interface: $F_m = F_i - (1 - C) F_{\text{ext}}$ in N.
    pub residual_clamp_force_n: f64,
    /// Axial tensile stress in the thread root $\sigma_b = F_b / A_s$ in MPa.
    pub bolt_axial_stress_mpa: f64,
    /// Safety factor against bolt proof load ($S_p A_s / F_b$).
    pub proof_safety_factor: f64,
    /// Load factor $n_0 = F_i / ((1 - C) F_{\text{ext}})$ against joint separation ($> 1.0$ means clamped).
    pub joint_separation_safety_factor: f64,
    /// Estimated assembly tightening torque $T = K d F_i$ (with nut factor $K = 0.20$) in $\text{N}\cdot\text{m}$.
    pub recommended_tightening_torque_nm: f64,
}

/// Performs a bolted joint preload & clamping analysis for an ISO metric bolt (`bolt_diameter_mm`, `pitch_mm`)
/// under initial preload `preload_n` and external separating load `external_axial_load_n` against `proof_strength_mpa`.
pub fn bolted_joint_analysis(
    bolt_diameter_mm: f64,
    pitch_mm: f64,
    preload_n: f64,
    external_axial_load_n: f64,
    proof_strength_mpa: f64,
) -> Result<BoltedJointReport> {
    if ![
        bolt_diameter_mm,
        pitch_mm,
        preload_n,
        external_axial_load_n,
        proof_strength_mpa,
    ]
    .into_iter()
    .all(f64::is_finite)
        || bolt_diameter_mm <= 0.5
        || pitch_mm <= 0.05
        || bolt_diameter_mm <= 1.1 * pitch_mm
        || preload_n <= 0.0
        || external_axial_load_n < 0.0
        || proof_strength_mpa <= 0.0
    {
        return Err(invalid(
            "MECHANICS_INVALID_INPUT",
            "Bolted joint analysis requires valid bolt diameter, pitch, positive preload, non-negative load, and positive proof strength",
        ));
    }
    let d_eff = bolt_diameter_mm - 0.9382 * pitch_mm;
    let a_s = 0.25 * std::f64::consts::PI * d_eff * d_eff;
    let c = 0.25_f64;
    let f_b = preload_n + c * external_axial_load_n;
    let f_m = preload_n - (1.0 - c) * external_axial_load_n;
    let sigma_b = f_b / a_s;
    let proof_load_n = proof_strength_mpa * a_s;
    let proof_fos = proof_load_n / f_b.max(1e-12);
    let sep_fos = if external_axial_load_n > 1e-12 {
        preload_n / ((1.0 - c) * external_axial_load_n)
    } else {
        f64::INFINITY
    };
    let torque_nm = 0.20 * (bolt_diameter_mm * 1e-3) * preload_n;
    Ok(BoltedJointReport {
        tensile_stress_area_mm2: a_s,
        stiffness_factor_c: c,
        bolt_axial_force_n: f_b,
        residual_clamp_force_n: f_m,
        bolt_axial_stress_mpa: sigma_b,
        proof_safety_factor: proof_fos,
        joint_separation_safety_factor: sep_fos,
        recommended_tightening_torque_nm: torque_nm,
    })
}

/// Euler–Johnson column buckling analysis report.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ColumnBucklingReport {
    /// Minimum radius of gyration $r = \sqrt{I_{\min} / A}$ in mm.
    pub radius_of_gyration_mm: f64,
    /// Effective column slenderness ratio $\lambda = K L / r$.
    pub slenderness_ratio: f64,
    /// Transition slenderness ratio $\lambda_c = \sqrt{2 \pi^2 E / \sigma_y}$ separating Johnson inelastic and Euler elastic buckling.
    pub transition_slenderness: f64,
    /// True if $\lambda \ge \lambda_c$ (Euler elastic regime), false if $\lambda < \lambda_c$ (Johnson parabolic regime).
    pub is_euler_regime: bool,
    /// Critical buckling stress $\sigma_{\text{cr}}$ in MPa.
    pub critical_stress_mpa: f64,
    /// Critical buckling load $P_{\text{cr}} = \sigma_{\text{cr}} A$ in N.
    pub critical_load_n: f64,
    /// Buckling safety factor $n = P_{\text{cr}} / P_{\text{applied}}$.
    pub safety_factor: f64,
}

/// Computes column buckling capacity across both intermediate (Johnson parabolic) and slender (Euler hyperbola)
/// regimes:
/// - Slenderness ratio $\lambda = \frac{K L}{\sqrt{I_{\min} / A}}$
/// - Transition slenderness $\lambda_c = \sqrt{\frac{2 \pi^2 E}{\sigma_y}}$
/// - Critical stress $\sigma_{\text{cr}} = \sigma_y \left(1 - \frac{\sigma_y \lambda^2}{4 \pi^2 E}\right)$ for $\lambda < \lambda_c$ (Johnson)
///   or $\sigma_{\text{cr}} = \frac{\pi^2 E}{\lambda^2}$ for $\lambda \ge \lambda_c$ (Euler).
pub fn column_buckling_euler_johnson(
    length_mm: f64,
    effective_length_factor_k: f64,
    area_mm2: f64,
    min_inertia_mm4: f64,
    elastic_modulus_mpa: f64,
    yield_strength_mpa: f64,
    applied_load_n: f64,
) -> Result<ColumnBucklingReport> {
    if ![
        length_mm,
        effective_length_factor_k,
        area_mm2,
        min_inertia_mm4,
        elastic_modulus_mpa,
        yield_strength_mpa,
    ]
    .into_iter()
    .all(|v| v.is_finite() && v > 0.0)
        || !applied_load_n.is_finite()
        || applied_load_n < 0.0
    {
        return Err(invalid(
            "MECHANICS_INVALID_INPUT",
            "Euler-Johnson buckling requires positive finite geometry/material properties and non-negative load",
        ));
    }
    let r_gyr = (min_inertia_mm4 / area_mm2).sqrt();
    let lambda = (effective_length_factor_k * length_mm) / r_gyr;
    let pi2_e = std::f64::consts::PI * std::f64::consts::PI * elastic_modulus_mpa;
    let lambda_c = (2.0 * pi2_e / yield_strength_mpa).sqrt();
    let is_euler = lambda >= lambda_c;
    let sigma_cr = if is_euler {
        pi2_e / (lambda * lambda)
    } else {
        yield_strength_mpa * (1.0 - (yield_strength_mpa * lambda * lambda) / (4.0 * pi2_e))
    };
    let p_cr = sigma_cr * area_mm2;
    let fos = if applied_load_n > 1e-12 {
        p_cr / applied_load_n
    } else {
        f64::INFINITY
    };
    Ok(ColumnBucklingReport {
        radius_of_gyration_mm: r_gyr,
        slenderness_ratio: lambda,
        transition_slenderness: lambda_c,
        is_euler_regime: is_euler,
        critical_stress_mpa: sigma_cr,
        critical_load_n: p_cr,
        safety_factor: fos,
    })
}

/// Computes cylindrical thin-walled pressure vessel stresses and von Mises safety factor:
/// returns `(hoop_stress_mpa, axial_stress_mpa, von_mises_mpa, safety_factor)` where
/// $\sigma_\theta = \frac{p r}{t}$, $\sigma_z = \frac{p r}{2 t}$, and
/// $\sigma_{\text{vM}} = \sqrt{\sigma_\theta^2 - \sigma_\theta \sigma_z + \sigma_z^2} = \frac{\sqrt{3}}{2} \sigma_\theta$.
pub fn thin_walled_pressure_vessel(
    inner_radius_mm: f64,
    wall_thickness_mm: f64,
    internal_pressure_mpa: f64,
    yield_strength_mpa: f64,
) -> Result<(f64, f64, f64, f64)> {
    if ![
        inner_radius_mm,
        wall_thickness_mm,
        internal_pressure_mpa,
        yield_strength_mpa,
    ]
    .into_iter()
    .all(f64::is_finite)
        || inner_radius_mm <= 0.0
        || wall_thickness_mm <= 0.0
        || internal_pressure_mpa < 0.0
        || yield_strength_mpa <= 0.0
    {
        return Err(invalid(
            "MECHANICS_INVALID_INPUT",
            "Thin-walled pressure vessel requires positive radius, thickness, yield strength, and non-negative pressure",
        ));
    }
    let hoop = internal_pressure_mpa * inner_radius_mm / wall_thickness_mm;
    let axial = 0.5 * hoop;
    let von_mises = (hoop * hoop - hoop * axial + axial * axial).sqrt();
    let safety_factor = if von_mises > 1e-12 {
        yield_strength_mpa / von_mises
    } else {
        f64::INFINITY
    };
    Ok((hoop, axial, von_mises, safety_factor))
}



#[path="lib/algorithm_min_wall_mm.rs"]
mod algorithm_min_wall_mm;
use algorithm_min_wall_mm::min_wall_mm;
#[path="lib/algorithm_analyze_section.rs"]
mod algorithm_analyze_section;
pub use algorithm_analyze_section::analyze_section;
