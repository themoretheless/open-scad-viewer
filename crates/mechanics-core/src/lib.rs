//! Strength-of-materials estimates from already-cut contours.
//!
//! Section estimates: area, centroid, second moments, section moduli,
//! a scanline wall-width probe, and σ = Mc/I (+ N/A). Not a surface kernel,
//! not a subdivision cage, not gear/thread generation (those live in ModelGraph).
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
pub mod print_strength;
pub mod thermal_strength;
pub mod truss;
pub mod truss_loads;

pub use math_core::{Error, Result};
use math_core::{cross2, sub2};
pub use planar_geometry::{LayerSection, MAX_LAYERS};

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
    let mut xs = Vec::new();
    let mut ys = Vec::new();
    for ring in &section.contours {
        for point in ring {
            xs.push(point[0]);
            ys.push(point[1]);
        }
    }
    if xs.is_empty() {
        return 0.0;
    }
    let min_x = xs.iter().copied().fold(f64::INFINITY, f64::min);
    let max_x = xs.iter().copied().fold(f64::NEG_INFINITY, f64::max);
    let min_y = ys.iter().copied().fold(f64::INFINITY, f64::min);
    let max_y = ys.iter().copied().fold(f64::NEG_INFINITY, f64::max);
    let span = (max_y - min_y).max(max_x - min_x);
    if !span.is_finite() || span <= 0.0 {
        return 0.0;
    }
    let step = (span / 32.0).max(span * 1e-4);
    let mut min_wall = f64::INFINITY;
    let mut y = min_y + step * 0.5;
    while y <= max_y - step * 0.25 {
        let mut hits = Vec::new();
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
    let mut layers = Vec::with_capacity(sections.len());
    for section in sections {
        layers.push(analyze_section(section, load)?);
    }
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
