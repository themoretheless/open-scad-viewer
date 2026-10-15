//! Shared helpers for lattice printability / strength screening reports.
use super::*;

/// Decimal rounding used by every screening report (matches the legacy TS `round`).
pub(super) fn rnd(v: f64, digits: i32) -> f64 {
    let s = 10f64.powi(digits);
    (v * s).round() / s
}

pub(super) fn is_spatial(pattern: &str) -> bool {
    matches!(pattern, "bone" | "spatial" | "bcc" | "octet")
}

pub(super) fn text(ru: impl Into<String>, en: impl Into<String>) -> Value {
    let (ru, en): (String, String) = (ru.into(), en.into());
    value_codec::json!({"ru": ru, "en": en})
}

/// Optional finite number with a default; non-finite or non-numeric values are refused.
pub(super) fn opt_num(v: &Value, key: &str, default: f64) -> Result<f64> {
    Ok(lattice_option(v, key)?.unwrap_or(default))
}

pub(super) fn opt_bool(v: &Value, key: &str) -> Result<bool> {
    match &v[key] {
        Value::Null => Ok(false),
        x => x
            .as_bool()
            .ok_or_else(|| input(format!("{key} must be boolean."))),
    }
}

pub(super) fn axis_index(axis: &str) -> usize {
    match axis {
        "x" => 0,
        "y" => 1,
        _ => 2,
    }
}

pub(super) fn axis_name(i: usize) -> &'static str {
    ["x", "y", "z"][i.min(2)]
}

/// Parsed subset of `LighteningOptions` used by the screening reports.
#[derive(Clone, Debug)]
pub(super) struct LatticeParams {
    pub pattern: String,
    pub axis: usize,
    pub cell: f64,
    pub rib: f64,
    pub rim: f64,
    pub bottom: f64,
    pub top: f64,
    pub skin: f64,
    pub open_top: bool,
    pub diagonals: bool,
    pub wall_depth: f64,
    pub keep_core: bool,
}

impl LatticeParams {
    pub fn parse(o: &Value) -> Result<Self> {
        if !o.is_object() {
            return Err(input("Lattice options must be an object."));
        }
        let pattern = o["pattern"]
            .as_str()
            .ok_or_else(|| input("Lattice pattern is required."))?
            .to_string();
        if !matches!(
            pattern.as_str(),
            "bone" | "spatial" | "bcc" | "octet" | "isogrid" | "grid" | "triangles" | "honeycomb" | "web"
        ) {
            return Err(input("Invalid lattice pattern."));
        }
        let axis = match o["axis"].as_str() {
            None => 2,
            Some(a @ ("x" | "y" | "z")) => axis_index(a),
            Some(_) => return Err(input("Lattice axis must be x, y or z.")),
        };
        let p = Self {
            axis,
            cell: required_lattice_option(o, "cell")?,
            rib: required_lattice_option(o, "rib")?,
            rim: opt_num(o, "rim", 0.)?,
            bottom: opt_num(o, "bottom", 0.)?,
            top: opt_num(o, "top", 0.)?,
            skin: opt_num(o, "skin", 0.)?,
            open_top: opt_bool(o, "openTop")?,
            diagonals: opt_bool(o, "diagonals")?,
            wall_depth: opt_num(o, "wallDepth", 0.)?,
            keep_core: opt_bool(o, "keepCore")?,
            pattern,
        };
        if p.cell <= 0. || p.rib <= 0. {
            return Err(input("Lattice cell and rib must be positive."));
        }
        Ok(p)
    }
    pub fn spatial(&self) -> bool {
        is_spatial(&self.pattern)
    }
}

/// Bounds of a flat `positions` array (xyz triples).
pub(super) fn mesh_bounds(mesh: &Mesh) -> Result<([f64; 3], [f64; 3])> {
    let (min, max) = polygon_core::scene_flatten::bounds(&[mesh.positions.clone()])?;
    if !(0..3).all(|k| min[k].is_finite() && max[k].is_finite() && max[k] >= min[k]) {
        return Err(input("Lattice body needs finite bounds."));
    }
    Ok((min, max))
}
