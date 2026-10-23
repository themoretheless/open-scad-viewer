//! Rolled I-section catalogue: standard designation → nominal dimensions →
//! computed geometric properties.
//!
//! Families: IPE (EN 10365), HEB (EN 10365), metric W-shapes (ASTM A6 /
//! CISC), GOST 8239-89 hot-rolled I-beams, STO ASChM 20-93 parallel-flange
//! I-beams (B / SH / K series).
//!
//! Properties are computed from the nominal dimensions by rectangular
//! decomposition — web plus two flange plates — with NO fillet (rolling
//! radius) material. Tabulated mill values therefore sit a few percent above
//! the computed ones (typically 3–7 % on area and strong-axis inertia).
//! GOST 8239-89 beams additionally have sloped flange inner faces; the
//! catalogue stores the *average* flange thickness, so the weak-axis inertia
//! of those profiles is overestimated by up to ~20 % — treat weak-axis
//! values of the `gost-8239` family as indicative only.
//!
//! Torsion constant J is the thin-walled strip sum (2·b·tf³ + (h−tf)·tw³)/3;
//! warping constant Cw = I_weak·(h−tf)²/4 (doubly symmetric I approximation).
//! This is an engineering estimate catalogue, not a certified section table.
use crate::{Error, Result};

fn unknown(designation: &str) -> Error {
    Error::new(
        "PROFILE_UNKNOWN",
        format!("Unknown structural profile designation: {designation}"),
    )
}

/// Nominal I-section dimensions, millimetres.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ISectionDims {
    /// Overall section depth.
    pub h_mm: f64,
    /// Flange width.
    pub b_mm: f64,
    /// Web thickness.
    pub tw_mm: f64,
    /// Flange thickness (average for sloped-flange GOST 8239-89 beams).
    pub tf_mm: f64,
}

/// Geometric properties of one catalogue profile, computed from the nominal
/// dimensions by rectangular decomposition (no fillets). Strong axis is the
/// centroidal axis parallel to the flanges; weak axis is perpendicular.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ProfileReport {
    /// Canonical designation as stored in the catalogue.
    pub designation: &'static str,
    /// Catalogue family: `ipe`, `heb`, `w`, `gost-8239`, `sto-aschm-b`,
    /// `sto-aschm-sh`, `sto-aschm-k`.
    pub family: &'static str,
    pub dims: ISectionDims,
    pub area_mm2: f64,
    /// Mass per metre at steel density 7850 kg/m³.
    pub mass_kg_m: f64,
    pub i_strong_mm4: f64,
    pub i_weak_mm4: f64,
    /// Elastic section moduli against the outer fibres.
    pub w_strong_mm3: f64,
    pub w_weak_mm3: f64,
    /// Saint-Venant torsion constant, strip sum.
    pub j_mm4: f64,
    /// Warping constant, doubly symmetric I approximation.
    pub cw_mm6: f64,
}

/// Geometric properties of an arbitrary doubly symmetric I-section from its
/// nominal dimensions; rectangular decomposition, no fillets.
pub fn i_section_properties(dims: &ISectionDims) -> Result<(f64, f64, f64, f64)> {
    let &ISectionDims {
        h_mm: h,
        b_mm: b,
        tw_mm: tw,
        tf_mm: tf,
    } = dims;
    if ![h, b, tw, tf].iter().all(|v| v.is_finite() && *v > 0.) || 2. * tf >= h || tw >= b {
        return Err(Error::new(
            "PROFILE_INVALID_DIMS",
            "I-section dimensions must be positive with 2·tf < h and tw < b",
        ));
    }
    let web_h = h - 2. * tf;
    let area = 2. * b * tf + web_h * tw;
    let i_strong = tw * web_h.powi(3) / 12.
        + 2. * (b * tf.powi(3) / 12. + b * tf * ((h - tf) / 2.).powi(2));
    let i_weak = 2. * tf * b.powi(3) / 12. + web_h * tw.powi(3) / 12.;
    let j = (2. * b * tf.powi(3) + (h - tf) * tw.powi(3)) / 3.;
    let cw = i_weak * (h - tf).powi(2) / 4.;
    if ![area, i_strong, i_weak, j, cw].iter().all(|v| v.is_finite()) {
        return Err(Error::new(
            "PROFILE_NUMERIC_RANGE",
            "Profile calculation exceeds finite numeric range",
        ));
    }
    Ok((area, i_strong, i_weak, j, cw))
}

/// One catalogue row: canonical designation, family, nominal dimensions.
struct CatalogRow {
    designation: &'static str,
    family: &'static str,
    dims: ISectionDims,
}

const fn row(designation: &'static str, family: &'static str, h: f64, b: f64, tw: f64, tf: f64) -> CatalogRow {
    CatalogRow {
        designation,
        family,
        dims: ISectionDims {
            h_mm: h,
            b_mm: b,
            tw_mm: tw,
            tf_mm: tf,
        },
    }
}

const IPE: &[CatalogRow] = &[
    row("IPE 80", "ipe", 80., 46., 3.8, 5.2),
    row("IPE 100", "ipe", 100., 55., 4.1, 5.7),
    row("IPE 120", "ipe", 120., 64., 4.4, 6.3),
    row("IPE 140", "ipe", 140., 73., 4.7, 6.9),
    row("IPE 160", "ipe", 160., 82., 5.0, 7.4),
    row("IPE 180", "ipe", 180., 91., 5.3, 8.0),
    row("IPE 200", "ipe", 200., 100., 5.6, 8.5),
    row("IPE 220", "ipe", 220., 110., 5.9, 9.2),
    row("IPE 240", "ipe", 240., 120., 6.2, 9.8),
    row("IPE 270", "ipe", 270., 135., 6.6, 10.2),
    row("IPE 300", "ipe", 300., 150., 7.1, 10.7),
];

const HEB: &[CatalogRow] = &[
    row("HEB 100", "heb", 100., 100., 6., 10.),
    row("HEB 120", "heb", 120., 120., 6.5, 11.),
    row("HEB 140", "heb", 140., 140., 7., 12.),
    row("HEB 160", "heb", 160., 160., 8., 13.),
    row("HEB 180", "heb", 180., 180., 8.5, 14.),
    row("HEB 200", "heb", 200., 200., 9., 15.),
    row("HEB 220", "heb", 220., 220., 9.5, 16.),
    row("HEB 240", "heb", 240., 240., 10., 17.),
    row("HEB 260", "heb", 260., 260., 10., 17.5),
    row("HEB 280", "heb", 280., 280., 10.5, 18.),
    row("HEB 300", "heb", 300., 300., 11., 19.),
];

const W: &[CatalogRow] = &[
    row("W150X22.5", "w", 152., 152., 5.8, 6.6),
    row("W200X22.5", "w", 206., 102., 6.2, 8.0),
    row("W250X32.7", "w", 258., 146., 6.1, 9.1),
    row("W310X38.7", "w", 310., 165., 5.8, 9.7),
    row("W310X52", "w", 317., 167., 7.6, 13.2),
];

/// GOST 8239-89 hot-rolled I-beams. These have sloped flange inner faces;
/// `tf` is the *average* flange thickness, so computed weak-axis inertia
/// overestimates the tabulated value (see module docs).
const GOST_8239: &[CatalogRow] = &[
    row("I10", "gost-8239", 100., 55., 4.5, 7.2),
    row("I12", "gost-8239", 120., 64., 4.8, 7.3),
    row("I14", "gost-8239", 140., 73., 4.9, 7.5),
    row("I16", "gost-8239", 160., 81., 5.0, 7.8),
    row("I18", "gost-8239", 180., 90., 5.1, 8.1),
    row("I20", "gost-8239", 200., 100., 5.2, 8.4),
    row("I22", "gost-8239", 220., 110., 5.4, 8.7),
    row("I24", "gost-8239", 240., 115., 5.6, 9.5),
    row("I27", "gost-8239", 270., 125., 6.0, 9.8),
    row("I30", "gost-8239", 300., 135., 6.5, 10.2),
];

/// STO ASChM 20-93 parallel-flange I-beams, normal (B) series.
const STO_B: &[CatalogRow] = &[
    row("10B1", "sto-aschm-b", 100., 55., 4.1, 5.7),
    row("12B1", "sto-aschm-b", 117.6, 64., 3.8, 5.1),
    row("12B2", "sto-aschm-b", 120., 64., 4.4, 6.3),
    row("14B1", "sto-aschm-b", 137.4, 73., 3.8, 5.6),
    row("14B2", "sto-aschm-b", 140., 73., 4.7, 6.9),
    row("16B1", "sto-aschm-b", 157., 82., 4.0, 5.9),
    row("16B2", "sto-aschm-b", 160., 82., 5.0, 7.4),
    row("18B1", "sto-aschm-b", 177., 91., 4.3, 6.5),
    row("18B2", "sto-aschm-b", 180., 91., 5.3, 8.0),
    row("20B1", "sto-aschm-b", 200., 100., 5.5, 8.0),
    row("25B1", "sto-aschm-b", 248., 124., 5.0, 8.0),
    row("25B2", "sto-aschm-b", 250., 125., 6.0, 9.0),
    row("30B1", "sto-aschm-b", 298., 149., 5.5, 8.0),
];

/// STO ASChM 20-93 wide-flange (SH) series.
const STO_SH: &[CatalogRow] = &[
    row("20SH1", "sto-aschm-sh", 194., 150., 6., 9.),
    row("25SH1", "sto-aschm-sh", 244., 175., 7., 11.),
    row("30SH1", "sto-aschm-sh", 294., 200., 8., 12.),
    row("30SH2", "sto-aschm-sh", 300., 201., 9., 15.),
];

/// STO ASChM 20-93 column (K) series.
const STO_K: &[CatalogRow] = &[
    row("20K1", "sto-aschm-k", 196., 199., 6.5, 10.),
    row("20K2", "sto-aschm-k", 200., 200., 8., 12.),
    row("25K1", "sto-aschm-k", 246., 249., 8., 12.),
    row("25K2", "sto-aschm-k", 250., 250., 9., 14.),
];

const CATALOG: &[&[CatalogRow]] = &[IPE, HEB, W, GOST_8239, STO_B, STO_SH, STO_K];

/// Normalizes a designation for lookup: uppercase, strips whitespace,
/// unifies ×/x/X, maps the Cyrillic homoglyphs used in GOST designations
/// (Б→B, Ш→SH, К→K, И→I).
fn normalize(designation: &str) -> String {
    let mut out = String::with_capacity(designation.len() + 1);
    for c in designation.chars().filter(|c| !c.is_whitespace()) {
        match c {
            '×' | 'х' | 'Х' => out.push('X'),
            'Б' => out.push('B'),
            'К' => out.push('K'),
            'И' => out.push('I'),
            'Ш' => out.push_str("SH"),
            c => out.extend(c.to_uppercase()),
        }
    }
    out
}

/// Every catalogue designation with its family and nominal dimensions, in
/// catalogue order — for UI pickers.
pub fn list() -> Vec<(&'static str, &'static str, ISectionDims)> {
    CATALOG
        .iter()
        .flat_map(|family| family.iter())
        .map(|r| (r.designation, r.family, r.dims))
        .collect()
}

/// Looks up one profile by designation (case/space/×-insensitive, Cyrillic
/// Б/Ш/К accepted) and computes its geometric properties from the nominal
/// dimensions. Errors: PROFILE_UNKNOWN.
pub fn lookup(designation: &str) -> Result<ProfileReport> {
    let key = normalize(designation);
    if key.is_empty() || key.len() > 32 {
        return Err(unknown(designation));
    }
    let row = CATALOG
        .iter()
        .flat_map(|family| family.iter())
        .find(|r| normalize(r.designation) == key)
        .ok_or_else(|| unknown(designation))?;
    let (area_mm2, i_strong_mm4, i_weak_mm4, j_mm4, cw_mm6) = i_section_properties(&row.dims)?;
    Ok(ProfileReport {
        designation: row.designation,
        family: row.family,
        dims: row.dims,
        area_mm2,
        mass_kg_m: area_mm2 * 0.00785,
        i_strong_mm4,
        i_weak_mm4,
        w_strong_mm3: i_strong_mm4 / (row.dims.h_mm / 2.),
        w_weak_mm3: i_weak_mm4 / (row.dims.b_mm / 2.),
        j_mm4,
        cw_mm6,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn close(actual: f64, reference: f64, rel_tol: f64) {
        assert!(
            (actual - reference).abs() <= rel_tol * reference.abs(),
            "{actual} vs {reference} (±{:.0}%)",
            rel_tol * 100.
        );
    }

    #[test]
    fn ipe_200_matches_en_10365_within_fillet_allowance() {
        // Tabulated: A = 28.5 cm², Ix = 1943 cm⁴, Iy = 142.4 cm⁴,
        // mass 22.4 kg/m. Computed values exclude fillets.
        let r = lookup("ipe200").unwrap();
        assert_eq!(r.designation, "IPE 200");
        assert_eq!(r.family, "ipe");
        close(r.area_mm2, 2850., 0.07);
        close(r.i_strong_mm4, 1943e4, 0.07);
        close(r.i_weak_mm4, 142.4e4, 0.07);
        close(r.mass_kg_m, 22.4, 0.07);
        close(r.w_strong_mm3, 194.3e3, 0.07);
    }

    #[test]
    fn heb_200_matches_en_10365_within_fillet_allowance() {
        // Tabulated: A = 78.1 cm², Ix = 5700 cm⁴, Iy = 2000 cm⁴.
        let r = lookup("HEB200").unwrap();
        close(r.area_mm2, 7810., 0.07);
        close(r.i_strong_mm4, 5700e4, 0.07);
        close(r.i_weak_mm4, 2000e4, 0.07);
    }

    #[test]
    fn w_shapes_match_aisc_metric_within_fillet_allowance() {
        // W250x32.7 (W10x22): A = 41.7 cm², Ix = 4900 cm⁴, Iy = 473 cm⁴,
        // mass 32.7 kg/m by designation.
        let r = lookup("W250×32.7").unwrap();
        assert_eq!(r.designation, "W250X32.7");
        close(r.area_mm2, 4170., 0.07);
        close(r.i_strong_mm4, 4900e4, 0.07);
        close(r.i_weak_mm4, 473e4, 0.07);
        close(r.mass_kg_m, 32.7, 0.07);
        // W310x52 (W12x35): A = 66.7 cm², Ix = 11900 cm⁴, Iy = 1020 cm⁴.
        let r = lookup("w310x52").unwrap();
        close(r.area_mm2, 6670., 0.07);
        close(r.i_strong_mm4, 11900e4, 0.07);
        close(r.i_weak_mm4, 1020e4, 0.07);
    }

    #[test]
    fn gost_8239_matches_area_and_strong_axis_but_not_weak_axis() {
        // I20 tabulated: A = 26.8 cm², Ix = 1840 cm⁴, Iy = 115 cm⁴.
        // Sloped flanges: computed Iy sits well above the table.
        let r = lookup("И20").unwrap();
        assert_eq!(r.designation, "I20");
        assert_eq!(r.family, "gost-8239");
        close(r.area_mm2, 2680., 0.07);
        close(r.i_strong_mm4, 1840e4, 0.07);
        assert!(r.i_weak_mm4 > 115e4);
        close(r.i_weak_mm4, 115e4, 0.30);
    }

    #[test]
    fn sto_aschm_b_matches_tabulated_values() {
        // 20Б1 tabulated: A = 27.16 cm², Ix = 1844 cm⁴, Iy = 133.9 cm⁴.
        let r = lookup("20Б1").unwrap();
        assert_eq!(r.designation, "20B1");
        close(r.area_mm2, 2716., 0.07);
        close(r.i_strong_mm4, 1844e4, 0.07);
        close(r.i_weak_mm4, 133.9e4, 0.07);
        // 30Б1: A = 40.6 cm², Ix = 6319 cm⁴, Iy = 441.9 cm⁴.
        let r = lookup("30б1").unwrap();
        close(r.area_mm2, 4060., 0.07);
        close(r.i_strong_mm4, 6319e4, 0.07);
        close(r.i_weak_mm4, 441.9e4, 0.07);
    }

    #[test]
    fn warping_and_torsion_constants_are_positive_and_plausible() {
        // IPE 300 handbook J ≈ 20.1 cm⁴, Cw ≈ 126e3 cm⁶; strip formulas
        // without fillets should land within ~25 %.
        let r = lookup("IPE 300").unwrap();
        close(r.j_mm4, 20.1e4, 0.30);
        close(r.cw_mm6, 126e9 * 1e-3, 0.30);
        assert!(r.j_mm4 > 0. && r.cw_mm6 > 0.);
    }

    #[test]
    fn lookup_rejects_unknown_and_empty_designations() {
        assert_eq!(lookup("").unwrap_err().code, "PROFILE_UNKNOWN");
        assert_eq!(lookup("IPE 999").unwrap_err().code, "PROFILE_UNKNOWN");
        assert_eq!(lookup("banana").unwrap_err().code, "PROFILE_UNKNOWN");
    }

    #[test]
    fn list_covers_all_families_without_duplicates() {
        let rows = list();
        assert_eq!(
            rows.len(),
            IPE.len() + HEB.len() + W.len() + GOST_8239.len() + STO_B.len() + STO_SH.len() + STO_K.len()
        );
        let mut keys: Vec<_> = rows.iter().map(|(d, _, _)| normalize(d)).collect();
        keys.sort();
        keys.dedup();
        assert_eq!(keys.len(), rows.len());
        // Every listed designation round-trips through lookup.
        for (designation, family, dims) in rows {
            let r = lookup(designation).unwrap();
            assert_eq!(r.family, family);
            assert_eq!(r.dims, dims);
        }
    }

    #[test]
    fn i_section_properties_refuses_degenerate_dimensions() {
        for dims in [
            ISectionDims { h_mm: 200., b_mm: 100., tw_mm: 5., tf_mm: 100. },
            ISectionDims { h_mm: 200., b_mm: 100., tw_mm: 0., tf_mm: 8. },
            ISectionDims { h_mm: 200., b_mm: 100., tw_mm: 100., tf_mm: 8. },
            ISectionDims { h_mm: f64::NAN, b_mm: 100., tw_mm: 5., tf_mm: 8. },
        ] {
            assert_eq!(
                i_section_properties(&dims).unwrap_err().code,
                "PROFILE_INVALID_DIMS"
            );
        }
    }
}
