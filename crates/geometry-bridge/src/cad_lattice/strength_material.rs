//! Material presets, structure classes and FDM service knock-downs for the
//! analytical lattice strength screen. Presets are typical nominal values,
//! not datasheet properties: callers may pass their own material object.
use super::report_util::*;
use super::*;
use value_codec::json;

#[derive(Clone, Debug)]
pub(super) struct Material {
    pub id: String,
    pub ru: String,
    pub en: String,
    pub e: f64,
    pub sigma_allow: f64,
    pub density: f64,
    pub fdm: bool,
    pub notch: f64,
    pub layer: f64,
    pub ductility: f64,
    pub e_parallel: f64,
    pub e_perp: f64,
    pub sigma_interlayer: f64,
    pub sigma_yield: f64,
    pub fatigue_ratio: f64,
    pub kt: f64,
}

type Preset = (&'static str, &'static str, &'static str, [f64; 12], bool);
// [E, σallow, ρ, notch, layer, ductility, E∥, E⊥, σinter, σy, fatigue, Kt]
const PRESETS: &[Preset] = &[
    ("pla", "PLA (FDM)", "PLA (FDM)", [3500., 35., 1.24, 1.35, 0.95, 0.25, 3500., 1800., 16., 40., 0.35, 2.2], true),
    ("petg", "PETG (FDM)", "PETG (FDM)", [2100., 30., 1.27, 1.1, 0.85, 0.45, 2100., 1200., 14., 35., 0.4, 1.9], true),
    ("abs", "ABS (FDM)", "ABS (FDM)", [2000., 25., 1.04, 1.2, 0.9, 0.4, 2000., 1100., 12., 30., 0.38, 2.0], true),
    ("nylon", "Nylon (FDM)", "Nylon (FDM)", [1700., 40., 1.14, 0.85, 0.75, 0.8, 1700., 900., 18., 48., 0.45, 1.6], true),
    ("al6061", "Алюминий 6061", "Aluminium 6061", [68900., 110., 2.7, 0.95, 0.05, 0.7, 68900., 68900., 110., 240., 0.4, 2.5], false),
    ("steel", "Сталь конструкционная", "Mild steel", [200000., 160., 7.85, 0.85, 0.02, 0.75, 200000., 200000., 160., 250., 0.4, 2.3], false),
];

impl Material {
    fn from_preset(p: &Preset) -> Self {
        let [e, sigma_allow, density, notch, layer, ductility, e_parallel, e_perp, sigma_interlayer, sigma_yield, fatigue_ratio, kt] = p.3;
        Self {
            id: p.0.into(), ru: p.1.into(), en: p.2.into(), e, sigma_allow, density, fdm: p.4,
            notch, layer, ductility, e_parallel, e_perp, sigma_interlayer, sigma_yield, fatigue_ratio, kt,
        }
    }
    pub fn preset(id: &str) -> Option<Self> {
        PRESETS.iter().find(|p| p.0 == id).map(Self::from_preset)
    }
    /// Either a preset id string or an explicit material object.
    pub fn parse(v: &Value) -> Result<Self> {
        if let Some(id) = v.as_str() {
            return Self::preset(id).ok_or_else(|| input("Unknown lattice material preset."));
        }
        if !v.is_object() {
            return Err(input("Material must be a preset id or an object."));
        }
        let e = required_lattice_option(v, "E")?;
        let sigma_allow = required_lattice_option(v, "sigmaAllow")?;
        let fdm = match v["process"].as_str() {
            Some("fdm") => true,
            Some("metal") => false,
            _ => return Err(input("Material process must be fdm or metal.")),
        };
        if e <= 0. || sigma_allow <= 0. {
            return Err(input("Material E and allowable stress must be positive."));
        }
        let m = Self {
            id: v["id"].as_str().unwrap_or("custom").into(),
            ru: v["ru"].as_str().unwrap_or("Материал").into(),
            en: v["en"].as_str().unwrap_or("Material").into(),
            e,
            sigma_allow,
            density: opt_num(v, "density", 1.)?,
            fdm,
            notch: opt_num(v, "notch", 1.)?,
            layer: opt_num(v, "layer", if fdm { 0.85 } else { 0.05 })?,
            ductility: opt_num(v, "ductility", 0.5)?,
            e_parallel: opt_num(v, "EParallel", e)?,
            e_perp: opt_num(v, "EPerp", if fdm { e * 0.55 } else { e })?,
            sigma_interlayer: opt_num(v, "sigmaInterlayer", if fdm { sigma_allow * 0.45 } else { sigma_allow })?,
            sigma_yield: opt_num(v, "sigmaYield", sigma_allow * 1.15)?,
            fatigue_ratio: opt_num(v, "fatigueRatio", 1.)?,
            kt: opt_num(v, "Kt", 1.)?,
        };
        if [m.e_parallel, m.e_perp, m.sigma_interlayer, m.sigma_yield, m.fatigue_ratio, m.kt]
            .iter()
            .any(|x| *x <= 0.)
            || !(0. ..=1.).contains(&m.layer)
            || !(0. ..=1.).contains(&m.ductility)
        {
            return Err(input("Material properties must be positive; layer and ductility within 0..1."));
        }
        Ok(m)
    }
    pub fn to_value(&self) -> Value {
        json!({"id": self.id.clone(), "ru": self.ru.clone(), "en": self.en.clone(), "E": self.e,
            "sigmaAllow": self.sigma_allow, "density": self.density,
            "process": if self.fdm { "fdm" } else { "metal" }, "notch": self.notch, "layer": self.layer,
            "ductility": self.ductility, "EParallel": self.e_parallel, "EPerp": self.e_perp,
            "sigmaInterlayer": self.sigma_interlayer, "sigmaYield": self.sigma_yield,
            "fatigueRatio": self.fatigue_ratio, "Kt": self.kt})
    }
}

/// `cad_lattice_materials`: nominal material presets.
pub fn materials(_: Value) -> Result<Value> {
    Ok(Value::Array(PRESETS.iter().map(|p| Material::from_preset(p).to_value()).collect()))
}

/// Qualitative structure class: (rank, load class, ru, en).
pub(super) fn structure_hint(pattern: &str) -> (u32, &'static str, &'static str, &'static str) {
    match pattern {
        "octet" => (1, "stretch", "Октет-ферма: растянутый каркас с лучшим отношением жёсткость/масса в объёме и в скелетных стенках.", "Octet truss: stretch-dominated; best stiffness-to-weight among volumetric and skeletal-wall modes."),
        "isogrid" => (2, "stretch", "Изогрид: равносторонние треугольные ячейки — классика облегчённых стенок (NASA isogrid).", "Isogrid: equilateral triangular cells — the classic lightened-wall layout (NASA isogrid)."),
        "bcc" => (3, "mixed", "ОЦК (BCC): стержни к центрам ячеек, хорошо держит сжатие при умеренном объёме.", "BCC: body-centered struts; strong in compression with moderate volume."),
        "triangles" => (4, "stretch", "Диагональный каркас: треугольники повышают жёсткость относительно прямоугольной сетки.", "Diagonal skeleton: triangulation stiffens the wall versus a plain grid."),
        "honeycomb" => (5, "mixed", "Соты: эффективны в плоскости панели, слабее на изгиб тонкой стенки.", "Honeycomb: efficient in-plane; weaker for thin-wall bending."),
        "spatial" => (6, "mixed", "Прямоугольная 3D-сеть; с диагоналями ближе к растянутому каркасу.", "Axis-aligned 3D grid; diagonals move it toward stretch-dominated behaviour."),
        "bone" => (7, "organic", "Кость: органическая пористость, не оптимизация по напряжениям.", "Bone: organic porosity, not stress-based topology optimization."),
        "web" => (8, "bending", "Случайная паутина: снижает объём, жёсткость непредсказуема.", "Random web: reduces volume; stiffness is unpredictable."),
        _ => (9, "bending", "Прямоугольная сетка: больше изгибных режимов, ниже удельная жёсткость.", "Rectangular grid: more bending modes and lower specific stiffness."),
    }
}

/// `cad_lattice_structure_hint`: `{pattern}` → `{rank, load, ru, en}`.
pub fn structure_hint_op(v: Value) -> Result<Value> {
    let pattern: String = field(&v, "pattern")?;
    LatticeParams::parse(&json!({"pattern": pattern.clone(), "cell": 1., "rib": 0.1}))?;
    let (rank, load, ru, en) = structure_hint(&pattern);
    Ok(json!({"rank": rank, "load": load, "ru": ru, "en": en}))
}

/// Gibson–Ashby exponent for relative modulus vs relative density.
pub(super) fn ashby_exponent(pattern: &str, diagonals: bool) -> f64 {
    match pattern {
        "octet" | "isogrid" | "triangles" => 1.,
        "bcc" => 1.4,
        "spatial" => if diagonals { 1.2 } else { 1.8 },
        "honeycomb" => 1.5,
        "bone" => 1.7,
        _ => match structure_hint(pattern).1 {
            "stretch" => 1.1,
            "bending" => 2.,
            _ => 1.6,
        },
    }
}

pub(super) struct Service {
    pub print_axis: usize,
    pub temperature_c: f64,
    pub humidity_pct: f64,
    pub infill: f64,
    pub perimeters: f64,
    pub min_wall_mm: f64,
    pub max_bridge_mm: f64,
}

impl Service {
    pub fn parse(v: &Value, default_axis: usize) -> Result<Self> {
        let empty = json!({});
        let v = if v.is_null() { &empty } else { v };
        let s = Self {
            print_axis: match v["printAxis"].as_str() {
                None => default_axis,
                Some(a @ ("x" | "y" | "z")) => axis_index(a),
                Some(_) => return Err(input("Print axis must be x, y or z.")),
            },
            temperature_c: opt_num(v, "temperatureC", 23.)?,
            humidity_pct: opt_num(v, "humidityPct", 50.)?,
            infill: opt_num(v, "infill", 1.)?,
            perimeters: opt_num(v, "perimeters", 3.)?,
            min_wall_mm: opt_num(v, "minWallMm", 0.8)?,
            max_bridge_mm: opt_num(v, "maxBridgeMm", 10.)?,
        };
        if !(0. ..=100.).contains(&s.humidity_pct) || !(0. ..=1.).contains(&s.infill) || s.perimeters < 0. {
            return Err(input("Check humidity (0..100 %), infill (0..1) and perimeters."));
        }
        Ok(s)
    }
}

pub(super) struct Effective {
    pub e: f64,
    pub e_parallel: f64,
    pub e_perp: f64,
    pub sigma_allow: f64,
    pub sigma_interlayer: f64,
    pub sigma_yield: f64,
    pub env: f64,
    pub layer_alignment: f64,
}

/// FDM orthotropy and service-condition knock-downs. `load_axis` is the load
/// direction; alignment with the print axis selects E⊥ / interlayer strength.
pub(super) fn effective(m: &Material, s: &Service, load_axis: [f64; 3]) -> Effective {
    let mut env = 1.;
    if m.fdm {
        if s.temperature_c > 40. {
            env *= (1. - (s.temperature_c - 40.) * 0.012).max(0.45);
        }
        if m.id == "nylon" && s.humidity_pct > 40. {
            env *= (1. - (s.humidity_pct - 40.) * 0.006).max(0.5);
        }
        env *= 0.55 + 0.45 * s.infill.clamp(0.15, 1.);
        env *= (0.7 + 0.1 * s.perimeters.max(1.)).min(1.);
    }
    let l = (load_axis[0].powi(2) + load_axis[1].powi(2) + load_axis[2].powi(2)).sqrt();
    let l = if l > 0. { l } else { 1. };
    let align = if m.fdm { (load_axis[s.print_axis] / l).abs().min(1.) } else { 0. };
    let (e_geo, sigma_geo) = if m.fdm {
        (
            m.e_parallel * (1. - align) + m.e_perp * align,
            m.sigma_allow.min(m.sigma_allow * (1. - align) + m.sigma_interlayer * align),
        )
    } else {
        (m.e, m.sigma_allow)
    };
    Effective {
        e: e_geo * env,
        e_parallel: m.e_parallel * env,
        e_perp: m.e_perp * env,
        sigma_allow: sigma_geo * env,
        sigma_interlayer: m.sigma_interlayer * env,
        sigma_yield: m.sigma_yield * env,
        env,
        layer_alignment: align,
    }
}
