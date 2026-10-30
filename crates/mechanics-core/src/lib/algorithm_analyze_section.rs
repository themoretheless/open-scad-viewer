//! lib algorithm: analyze_section.
use super::*;
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
