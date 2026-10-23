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
pub mod thermal_strength;
pub mod truss;
pub mod truss_loads;

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

fn min_wall_mm(section: &LayerSection) -> f64 {
    let mut min_x = f64::INFINITY;
    let mut max_x = f64::NEG_INFINITY;
    let mut min_y = f64::INFINITY;
    let mut max_y = f64::NEG_INFINITY;
    let mut has_points = false;
    for ring in &section.contours {
        for point in ring {
            has_points = true;
            min_x = min_x.min(point[0]);
            max_x = max_x.max(point[0]);
            min_y = min_y.min(point[1]);
            max_y = max_y.max(point[1]);
        }
    }
    if !has_points {
        return 0.0;
    }
    let span = (max_y - min_y).max(max_x - min_x);
    if !span.is_finite() || span <= 0.0 {
        return 0.0;
    }
    let mut min_wall = f64::INFINITY;
    // Rotation-invariant normal ray probes from edge midpoints
    for (ring_idx, ring) in section.contours.iter().enumerate() {
        for i in 0..ring.len() {
            let a = ring[i];
            let b = ring[(i + 1) % ring.len()];
            let dx = b[0] - a[0];
            let dy = b[1] - a[1];
            let len = dx.hypot(dy);
            if len <= 1e-9 {
                continue;
            }
            let mid = [(a[0] + b[0]) * 0.5, (a[1] + b[1]) * 0.5];
            for sign in [1.0, -1.0] {
                let dir = [-sign * dy / len, sign * dx / len];
                let probe = [
                    mid[0] + dir[0] * (span * 1e-5),
                    mid[1] + dir[1] * (span * 1e-5),
                ];
                if winding(probe, &section.contours) == 0 {
                    continue;
                }
                let mut best_t = f64::INFINITY;
                for (other_ring_idx, other_ring) in section.contours.iter().enumerate() {
                    for j in 0..other_ring.len() {
                        if ring_idx == other_ring_idx && i == j {
                            continue;
                        }
                        let c = other_ring[j];
                        let d = other_ring[(j + 1) % other_ring.len()];
                        let s = [d[0] - c[0], d[1] - c[1]];
                        let denom = cross2(dir, s);
                        if denom.abs() <= 1e-12 {
                            continue;
                        }
                        let qp = sub2(c, mid);
                        let t = cross2(qp, s) / denom;
                        let u = cross2(qp, dir) / denom;
                        if t > 1e-7 && (0.0..=1.0).contains(&u) && t < best_t {
                            let check_mid =
                                [mid[0] + dir[0] * (t * 0.5), mid[1] + dir[1] * (t * 0.5)];
                            if winding(check_mid, &section.contours) != 0 {
                                best_t = t;
                            }
                        }
                    }
                }
                if best_t.is_finite() {
                    min_wall = min_wall.min(best_t);
                }
            }
        }
    }
    if min_wall.is_finite() {
        return min_wall;
    }
    let step = (span / 32.0).max(span * 1e-4);
    let mut y = min_y + step * 0.5;
    let mut hits = Vec::new();
    while y <= max_y - step * 0.25 {
        hits.clear();
        for ring in &section.contours {
            for i in 0..ring.len() {
                let a = ring[i];
                let b = ring[(i + 1) % ring.len()];
                let (y0, y1) = (a[1], b[1]);
                if (y0 <= y && y < y1) || (y1 <= y && y < y0) {
                    let t = (y - y0) / (y1 - y0);
                    if t.is_finite() {
                        hits.push(a[0] + (b[0] - a[0]) * t);
                    }
                }
            }
        }
        hits.sort_by(|a, b| a.total_cmp(b));
        let mut pair = 0;
        while pair + 1 < hits.len() {
            let x0 = hits[pair];
            let x1 = hits[pair + 1];
            let mid = [(x0 + x1) * 0.5, y];
            if winding(mid, &section.contours) != 0 {
                min_wall = min_wall.min((x1 - x0).abs());
            }
            pair += 2;
        }
        y += step;
    }
    if min_wall.is_finite() { min_wall } else { 0.0 }
}

/// Second moments and stress estimates for one already-cut section.
pub fn analyze_section(section: &LayerSection, load: &LoadCase) -> Result<SectionReport> {
    require_section(section)?;
    require_load(load)?;
    if section.contours.is_empty() {
        return Err(invalid(
            "MECHANICS_EMPTY_SECTION",
            "Section has no contours",
        ));
    }
    let mut twice_area = 0.0;
    let mut cx = 0.0;
    let mut cy = 0.0;
    let mut ixx_o = 0.0;
    let mut iyy_o = 0.0;
    let mut ixy_o = 0.0;
    for ring in &section.contours {
        let (da, dcx, dcy, dixx, diyy, dixy) = polygon_integrals(ring);
        twice_area += da;
        cx += dcx;
        cy += dcy;
        ixx_o += dixx;
        iyy_o += diyy;
        ixy_o += dixy;
    }
    let area = twice_area * 0.5;
    if !area.is_finite() || area.abs() < 1e-12 {
        return Err(invalid(
            "MECHANICS_EMPTY_SECTION",
            "Section area is degenerate",
        ));
    }
    let centroid = [cx / (6.0 * area), cy / (6.0 * area)];
    let ixx = ixx_o / 12.0 - area * centroid[1] * centroid[1];
    let iyy = iyy_o / 12.0 - area * centroid[0] * centroid[0];
    let ixy = ixy_o / 24.0 - area * centroid[0] * centroid[1];
    if !ixx.is_finite() || !iyy.is_finite() || !ixy.is_finite() {
        return Err(invalid(
            "MECHANICS_NUMERIC",
            "Section moments exceeded finite bounds",
        ));
    }
    let (max_x, max_y) = extrema(section, centroid);
    let wx = if max_y > 0.0 { ixx.abs() / max_y } else { 0.0 };
    let wy = if max_x > 0.0 { iyy.abs() / max_x } else { 0.0 };
    let bending = if wx > 0.0 {
        load.moment_x_nmm.abs() / wx
    } else {
        0.0
    } + if wy > 0.0 {
        load.moment_y_nmm.abs() / wy
    } else {
        0.0
    };
    let axial = load.axial_n / area;
    let von_mises = (bending + axial.abs()).abs();
    let fos = if von_mises > 0.0 {
        load.allowable_mpa / von_mises
    } else {
        f64::INFINITY
    };
    Ok(SectionReport {
        z_mm: section.z_mm,
        area_mm2: area.abs(),
        centroid,
        ixx_mm4: ixx.abs(),
        iyy_mm4: iyy.abs(),
        ixy_mm4: ixy,
        wx_mm3: wx,
        wy_mm3: wy,
        min_wall_mm: min_wall_mm(section),
        bending_mpa: bending,
        axial_mpa: axial,
        von_mises_mpa: von_mises,
        factor_of_safety: fos,
    })
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
mod lib_tests {
    use super::*;

    #[test]
    fn min_wall_mm_is_rotation_invariant_for_tilted_thin_strips() {
        // A 20 mm x 1.2 mm thin strip rotated by 37 degrees
        let angle = 37.0_f64.to_radians();
        let (s, c) = angle.sin_cos();
        let rot = |x: f64, y: f64| [x * c - y * s, x * s + y * c];
        let rect = vec![
            rot(0.0, 0.0),
            rot(20.0, 0.0),
            rot(20.0, 1.2),
            rot(0.0, 1.2),
        ];
        let sec = LayerSection {
            z_mm: 0.0,
            contours: vec![rect],
        };
        let wall = min_wall_mm(&sec);
        assert!((wall - 1.2).abs() < 1e-4, "expected 1.2 mm, got {wall}");
    }

    #[test]
    fn wall_thickness_3d_detects_thin_box_walls() {
        // Closed box: 10 x 10 x 1.5 mm
        let positions = vec![
            0.0, 0.0, 0.0, 10.0, 0.0, 0.0, 10.0, 10.0, 0.0, 0.0, 10.0, 0.0, 0.0, 0.0, 1.5, 10.0,
            0.0, 1.5, 10.0, 10.0, 1.5, 0.0, 10.0, 1.5,
        ];
        let indices = vec![
            0, 2, 1, 0, 3, 2, // bottom (-Z)
            4, 5, 6, 4, 6, 7, // top (+Z)
            0, 1, 5, 0, 5, 4, // front (-Y)
            2, 3, 7, 2, 7, 6, // back (+Y)
            0, 4, 7, 0, 7, 3, // left (-X)
            1, 2, 6, 1, 6, 5, // right (+X)
        ];
        let rep = analyze_wall_thickness_3d(&positions, &indices, 2.0).unwrap();
        assert!((rep.min_thickness_mm - 1.5).abs() < 1e-6);
        assert!(!rep.thin_triangle_indices.is_empty());
    }

    #[test]
    fn euler_buckling_rayleigh_frequency_and_fatigue() {
        let (p_cr, fos) = euler_buckling_safety_factor(200_000.0, 1000.0, 500.0, 1.0, 2000.0).unwrap();
        assert!(p_cr > 7800.0 && p_cr < 8000.0);
        assert!(fos > 3.9);

        // Single DOF 1 mm static deflection under gravity -> f = (1/2pi)*sqrt(9.80665 / 0.001) ≈ 15.76 Hz
        let f1 = estimate_fundamental_frequency_hz(&[1.0], &[1.0]).unwrap();
        assert!((f1 - 15.756).abs() < 0.05);

        let n_goodman = fatigue_safety_factor(50.0, 150.0, 200.0, 350.0, 500.0, FatigueCriterion::Goodman).unwrap();
        let n_soderberg = fatigue_safety_factor(50.0, 150.0, 200.0, 350.0, 500.0, FatigueCriterion::Soderberg).unwrap();
        let n_gerber = fatigue_safety_factor(50.0, 150.0, 200.0, 350.0, 500.0, FatigueCriterion::Gerber).unwrap();
        assert!(n_soderberg < n_goodman && n_goodman < n_gerber);
        assert!((n_goodman - 2.2222).abs() < 1e-3);

        // M8 x 1.25 class 8.8 bolt (proof strength 600 MPa) with 12 kN preload and 5 kN external load
        let bolt = bolted_joint_analysis(8.0, 1.25, 12_000.0, 5_000.0, 600.0).unwrap();
        assert!((bolt.tensile_stress_area_mm2 - 36.6).abs() < 0.1);
        assert!(bolt.proof_safety_factor > 1.5);
        assert!(bolt.joint_separation_safety_factor > 2.5);
        assert!((bolt.recommended_tightening_torque_nm - 19.2).abs() < 0.5);

        // 20 mm solid shaft pressed into a 40 mm OD hub (length 25 mm, diametral interference 0.03 mm, steel E=200 GPa)
        let pf = press_fit_interference_analysis(
            20.0,
            20.0,
            40.0,
            25.0,
            0.03,
            200e9,
            0.3,
            0.15,
        )
        .unwrap();
        // Contact pressure p = E * delta / (2 * d) * (1 - (d/D)^2) = 200000 * 0.03 / 40 * 0.75 = 112.5 MPa
        assert!((pf.contact_pressure_mpa - 112.5).abs() < 1e-3);
        // Max hoop stress at bore = p * (D^2 + d^2) / (D^2 - d^2) = 112.5 * (5/3) = 187.5 MPa
        assert!((pf.max_hub_hoop_stress_mpa - 187.5).abs() < 1e-3);
        assert!(pf.axial_assembly_force_n > 20_000.0);
        assert!(pf.max_transmitted_torque_nm > 200.0);

        // 20 mm solid steel shaft (L = 200 mm, T = 100 N*m, G = 80 GPa, tau_y = 250 MPa)
        let shaft = torsion_shaft_analysis(20.0, 0.0, 200.0, 100.0, 80e9, 250.0).unwrap();
        // J = pi/32 * 20^4 = 5000 * pi ≈ 15707.96 mm^4, tau_max = 100000 * 10 / J ≈ 63.662 MPa
        assert!((shaft.polar_moment_j_mm4 - 5000.0 * std::f64::consts::PI).abs() < 1e-3);
        assert!((shaft.max_shear_stress_mpa - 63.661977).abs() < 1e-3);
        assert!((shaft.safety_factor - 250.0 / 63.661977).abs() < 1e-3);
        assert!(shaft.twist_angle_deg > 0.5 && shaft.twist_angle_deg < 1.5);

        // Hertzian contact: two identical steel spheres (R1 = R2 = 10 mm, F = 1000 N, E = 200 GPa, nu = 0.3)
        let hc_sph =
            hertz_contact_sphere_sphere(10.0, 10.0, 1000.0, 200e9, 0.3, 200e9, 0.3).unwrap();
        // R* = 5 mm, E* = 109890.1 MPa -> a ≈ 0.3245 mm, p_max ≈ 4532.9 MPa
        assert!(hc_sph.contact_half_width_a_mm > 0.25 && hc_sph.contact_half_width_a_mm < 0.45);
        assert!(hc_sph.max_contact_pressure_mpa > 4000.0 && hc_sph.max_contact_pressure_mpa < 5000.0);
        assert!(hc_sph.approach_deflection_mm > 0.01 && hc_sph.approach_deflection_mm < 0.05);
        assert!((hc_sph.max_subsurface_shear_stress_mpa - 0.31 * hc_sph.max_contact_pressure_mpa).abs() < 1e-6);

        // Hertzian line contact: steel roller (R = 10 mm, L = 20 mm) on flat steel plane (F = 2000 N, E* = 115.38 GPa)
        let hc_cyl = hertz_contact_cylinder_plane(10.0, 20.0, 2000.0, 115.3846e9).unwrap();
        assert!(hc_cyl.contact_half_width_a_mm > 0.05 && hc_cyl.contact_half_width_a_mm < 0.25);
        assert!(hc_cyl.max_contact_pressure_mpa > 400.0 && hc_cyl.max_contact_pressure_mpa < 1200.0);
        assert!((hc_cyl.max_subsurface_shear_stress_mpa - 0.30 * hc_cyl.max_contact_pressure_mpa).abs() < 1e-6);
    }
}

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

/// Lamé thick-walled cylinder interference / press-fit stress and capacity report.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PressFitReport {
    /// Interface contact pressure $p$ in MPa ($\text{N/mm}^2$).
    pub contact_pressure_mpa: f64,
    /// Maximum tensile tangential (hoop) stress at the hub inner bore $\sigma_{\theta,\max}$ in MPa.
    pub max_hub_hoop_stress_mpa: f64,
    /// Equivalent von Mises stress at the hub inner bore $\sigma_{\text{vm}} = \sqrt{\sigma_\theta^2 + \sigma_r^2 - \sigma_\theta \sigma_r}$ in MPa.
    pub hub_bore_von_mises_mpa: f64,
    /// Axial force required to press-assemble or strip the joint $F_{\text{axial}} = \mu p \pi d L$ in N.
    pub axial_assembly_force_n: f64,
    /// Maximum slip-free transmitted torque capacity $T_{\max} = F_{\text{axial}} (d / 2)$ in $\text{N}\cdot\text{m}$.
    pub max_transmitted_torque_nm: f64,
}

/// Computes Lamé thick-walled cylinder interference press-fit contact pressure $p$, hub inner-bore
/// hoop stress $\sigma_\theta$, axial assembly force $F_{\text{axial}}$, and slip torque capacity $T_{\max}$
/// for a solid shaft of diameter `shaft_outer_dia_mm` pressed into a hub (`hub_inner_dia_mm`, `hub_outer_dia_mm`)
/// with diametral interference `interference_mm` (or if `interference_mm <= 0.0`, derived from `shaft_outer_dia_mm - hub_inner_dia_mm`).
pub fn press_fit_interference_analysis(
    shaft_outer_dia_mm: f64,
    hub_inner_dia_mm: f64,
    hub_outer_dia_mm: f64,
    engagement_length_mm: f64,
    interference_mm: f64,
    youngs_modulus_pa: f64,
    poisson_ratio: f64,
    friction_coeff: f64,
) -> Result<PressFitReport> {
    if ![
        shaft_outer_dia_mm,
        hub_inner_dia_mm,
        hub_outer_dia_mm,
        engagement_length_mm,
        interference_mm,
        youngs_modulus_pa,
        poisson_ratio,
        friction_coeff,
    ]
    .into_iter()
    .all(f64::is_finite)
        || shaft_outer_dia_mm <= 0.5
        || hub_inner_dia_mm <= 0.5
        || hub_outer_dia_mm <= hub_inner_dia_mm * 1.05
        || engagement_length_mm <= 0.1
        || youngs_modulus_pa <= 1e6
        || !(0.0..0.5).contains(&poisson_ratio)
        || friction_coeff <= 0.0
    {
        return Err(invalid(
            "MECHANICS_INVALID_INPUT",
            "Press-fit analysis requires valid positive geometry, hub_outer > 1.05 * hub_inner, E > 1 MPa, 0 <= nu < 0.5, and mu > 0",
        ));
    }
    let delta_d = if interference_mm > 0.0 {
        interference_mm
    } else {
        shaft_outer_dia_mm - hub_inner_dia_mm
    };
    if delta_d <= 0.0 {
        return Err(invalid(
            "MECHANICS_INVALID_INPUT",
            "Diametral interference must be positive",
        ));
    }
    let d = 0.5 * (shaft_outer_dia_mm + hub_inner_dia_mm);
    let d_outer = hub_outer_dia_mm;
    let e_mpa = youngs_modulus_pa * 1e-6;
    let ratio_sq = (d / d_outer).powi(2);
    // Identical-modulus Lamé equation for solid shaft in thick-walled hub (radial interference = delta_d / 2):
    // p = (E * delta_d / (2 * d)) * (1 - (d / D_outer)^2)
    let p_mpa = (e_mpa * delta_d / (2.0 * d)) * (1.0 - ratio_sq);
    let k_hoop = (1.0 + ratio_sq) / (1.0 - ratio_sq);
    let sigma_theta_mpa = p_mpa * k_hoop;
    let sigma_r_mpa = -p_mpa;
    let vm_mpa = (sigma_theta_mpa * sigma_theta_mpa
        + sigma_r_mpa * sigma_r_mpa
        - sigma_theta_mpa * sigma_r_mpa)
        .sqrt();
    let contact_area_mm2 = std::f64::consts::PI * d * engagement_length_mm;
    let axial_force_n = friction_coeff * p_mpa * contact_area_mm2;
    let torque_nm = axial_force_n * (0.5 * d * 1e-3);
    Ok(PressFitReport {
        contact_pressure_mpa: p_mpa,
        max_hub_hoop_stress_mpa: sigma_theta_mpa,
        hub_bore_von_mises_mpa: vm_mpa,
        axial_assembly_force_n: axial_force_n,
        max_transmitted_torque_nm: torque_nm,
    })
}

/// Circular solid or hollow shaft torsion stress, twist angle, and stiffness report.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TorsionShaftReport {
    /// Polar moment of inertia $J = \frac{\pi}{32}(d_o^4 - d_i^4)$ in $\text{mm}^4$.
    pub polar_moment_j_mm4: f64,
    /// Polar section modulus $Z_p = J / (d_o / 2)$ in $\text{mm}^3$.
    pub polar_section_modulus_mm3: f64,
    /// Maximum outer-fiber torsional shear stress $\tau_{\max} = T r_o / J$ in MPa.
    pub max_shear_stress_mpa: f64,
    /// Total angle of twist $\theta = T L / (G J)$ in radians.
    pub twist_angle_rad: f64,
    /// Total angle of twist $\theta$ in degrees.
    pub twist_angle_deg: f64,
    /// Torsional spring stiffness $k_t = G J / L$ in $\text{N}\cdot\text{m/rad}$.
    pub torsional_stiffness_nm_per_rad: f64,
    /// Torsional yield safety factor $\tau_{\text{yield}} / \tau_{\max}$.
    pub safety_factor: f64,
}

/// Analyzes a solid (`inner_dia_mm == 0.0`) or hollow (`0.0 <= inner_dia_mm < outer_dia_mm`)
/// circular shaft of length `length_mm` under torque `torque_nm` ($\text{N}\cdot\text{m}$),
/// shear modulus `shear_modulus_pa` ($\text{Pa}$), and allowable shear yield strength `yield_shear_mpa` ($\text{MPa}$).
pub fn torsion_shaft_analysis(
    outer_dia_mm: f64,
    inner_dia_mm: f64,
    length_mm: f64,
    torque_nm: f64,
    shear_modulus_pa: f64,
    yield_shear_mpa: f64,
) -> Result<TorsionShaftReport> {
    if ![
        outer_dia_mm,
        inner_dia_mm,
        length_mm,
        torque_nm,
        shear_modulus_pa,
        yield_shear_mpa,
    ]
    .into_iter()
    .all(f64::is_finite)
        || outer_dia_mm <= 0.1
        || inner_dia_mm < 0.0
        || inner_dia_mm >= outer_dia_mm
        || length_mm <= 0.1
        || shear_modulus_pa <= 1e6
        || yield_shear_mpa <= 0.0
    {
        return Err(invalid(
            "MECHANICS_INVALID_INPUT",
            "Torsion shaft analysis requires 0 <= inner_dia_mm < outer_dia_mm, length > 0.1 mm, G > 1 MPa, and yield_shear_mpa > 0",
        ));
    }
    let j_mm4 = (std::f64::consts::PI / 32.0) * (outer_dia_mm.powi(4) - inner_dia_mm.powi(4));
    let r_o_mm = 0.5 * outer_dia_mm;
    let zp_mm3 = j_mm4 / r_o_mm;
    let torque_nmm = torque_nm.abs() * 1e3;
    let tau_max_mpa = torque_nmm / zp_mm3;
    let g_mpa = shear_modulus_pa * 1e-6;
    let theta_rad = (torque_nmm * length_mm) / (g_mpa * j_mm4);
    let kt_nm_per_rad = (g_mpa * j_mm4 / length_mm) * 1e-3;
    let fos = if tau_max_mpa > 1e-12 {
        yield_shear_mpa / tau_max_mpa
    } else {
        f64::INFINITY
    };
    Ok(TorsionShaftReport {
        polar_moment_j_mm4: j_mm4,
        polar_section_modulus_mm3: zp_mm3,
        max_shear_stress_mpa: tau_max_mpa,
        twist_angle_rad: theta_rad,
        twist_angle_deg: theta_rad.to_degrees(),
        torsional_stiffness_nm_per_rad: kt_nm_per_rad,
        safety_factor: fos,
    })
}

/// Hertzian contact mechanics report for spherical (point) or cylindrical (line) non-conformal contact.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct HertzContactReport {
    /// Contact patch radius (sphere-sphere) or rectangular half-width (cylinder-plane) $a$ in mm.
    pub contact_half_width_a_mm: f64,
    /// Peak Hertzian normal contact pressure $p_{\max}$ at the center of the contact patch in MPa.
    pub max_contact_pressure_mpa: f64,
    /// Mean contact pressure $p_{\text{mean}}$ across the contact patch in MPa.
    pub mean_contact_pressure_mpa: f64,
    /// Mutual normal approach / elastic indentation deflection $\delta$ in mm.
    pub approach_deflection_mm: f64,
    /// Maximum subsurface Tresca / principal shear stress $\tau_{\max} \approx 0.30\,p_{\max}$ in MPa.
    pub max_subsurface_shear_stress_mpa: f64,
    /// Depth below the contact surface at which $\tau_{\max}$ occurs ($z_{\tau} \approx 0.48\,a$ or $0.78\,a$) in mm.
    pub shear_stress_depth_mm: f64,
}

/// Computes Hertzian point-contact stress and deflection between two spheres of radii `r1_mm` and `r2_mm`
/// under normal load `normal_force_n` ($\text{N}$) with elastic moduli `e1_pa`, `e2_pa` ($\text{Pa}$)
/// and Poisson ratios `nu1`, `nu2`.
pub fn hertz_contact_sphere_sphere(
    r1_mm: f64,
    r2_mm: f64,
    normal_force_n: f64,
    e1_pa: f64,
    nu1: f64,
    e2_pa: f64,
    nu2: f64,
) -> Result<HertzContactReport> {
    if ![r1_mm, r2_mm, normal_force_n, e1_pa, nu1, e2_pa, nu2]
        .into_iter()
        .all(f64::is_finite)
        || r1_mm <= 0.05
        || r2_mm <= 0.05
        || normal_force_n <= 0.0
        || e1_pa <= 1e6
        || e2_pa <= 1e6
        || !(0.0..0.5).contains(&nu1)
        || !(0.0..0.5).contains(&nu2)
    {
        return Err(invalid(
            "MECHANICS_INVALID_INPUT",
            "Sphere-sphere Hertzian contact requires positive radii, positive force, E > 1 MPa, and 0 <= nu < 0.5",
        ));
    }
    let r_eff_mm = (r1_mm * r2_mm) / (r1_mm + r2_mm);
    let e1_mpa = e1_pa * 1e-6;
    let e2_mpa = e2_pa * 1e-6;
    let inv_e_star = (1.0 - nu1 * nu1) / e1_mpa + (1.0 - nu2 * nu2) / e2_mpa;
    let e_star_mpa = 1.0 / inv_e_star;

    let a_mm = ((3.0 * normal_force_n * r_eff_mm) / (4.0 * e_star_mpa)).cbrt();
    let area_mm2 = std::f64::consts::PI * a_mm * a_mm;
    let p_mean_mpa = normal_force_n / area_mm2.max(1e-18);
    let p_max_mpa = 1.5 * p_mean_mpa;
    let delta_mm = (a_mm * a_mm) / r_eff_mm;
    let tau_max_mpa = 0.31 * p_max_mpa;
    let depth_mm = 0.48 * a_mm;

    Ok(HertzContactReport {
        contact_half_width_a_mm: a_mm,
        max_contact_pressure_mpa: p_max_mpa,
        mean_contact_pressure_mpa: p_mean_mpa,
        approach_deflection_mm: delta_mm,
        max_subsurface_shear_stress_mpa: tau_max_mpa,
        shear_stress_depth_mm: depth_mm,
    })
}

/// Computes Hertzian line-contact stress and deflection for a cylinder of radius `radius_mm` and
/// contact length `length_mm` pressed against a flat plane under normal force `normal_force_n` ($\text{N}$)
/// with effective contact modulus $E^*$ (`e_eff_pa` in $\text{Pa}$).
pub fn hertz_contact_cylinder_plane(
    radius_mm: f64,
    length_mm: f64,
    normal_force_n: f64,
    e_eff_pa: f64,
) -> Result<HertzContactReport> {
    if ![radius_mm, length_mm, normal_force_n, e_eff_pa]
        .into_iter()
        .all(f64::is_finite)
        || radius_mm <= 0.05
        || length_mm <= 0.1
        || normal_force_n <= 0.0
        || e_eff_pa <= 1e6
    {
        return Err(invalid(
            "MECHANICS_INVALID_INPUT",
            "Cylinder-plane Hertzian contact requires positive radius, length, normal force, and e_eff_pa > 1 MPa",
        ));
    }
    let e_star_mpa = e_eff_pa * 1e-6;
    let line_load_n_per_mm = normal_force_n / length_mm;
    let a_mm = ((4.0 * line_load_n_per_mm * radius_mm) / (std::f64::consts::PI * e_star_mpa)).sqrt();
    let p_max_mpa = (2.0 * line_load_n_per_mm) / (std::f64::consts::PI * a_mm.max(1e-12));
    let p_mean_mpa = line_load_n_per_mm / (2.0 * a_mm.max(1e-12));
    // Johnson / Roark cylindrical line-contact approach approximation
    let ratio = (4.0 * radius_mm / a_mm.max(1e-9)).max(2.0);
    let delta_mm = (2.0 * line_load_n_per_mm / (std::f64::consts::PI * e_star_mpa))
        * (ratio.ln() - 0.5).max(0.5);
    let tau_max_mpa = 0.30 * p_max_mpa;
    let depth_mm = 0.78 * a_mm;

    Ok(HertzContactReport {
        contact_half_width_a_mm: a_mm,
        max_contact_pressure_mpa: p_max_mpa,
        mean_contact_pressure_mpa: p_mean_mpa,
        approach_deflection_mm: delta_mm,
        max_subsurface_shear_stress_mpa: tau_max_mpa,
        shear_stress_depth_mm: depth_mm,
    })
}




