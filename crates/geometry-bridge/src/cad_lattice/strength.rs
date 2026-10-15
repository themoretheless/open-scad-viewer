//! Analytical strength-of-materials screen for a lightened body:
//! Gibson–Ashby scaling, strut axial / Euler / eccentric-bending checks,
//! FDM orthotropy and service knock-downs, panel buckling, design advice
//! and ranked weak spots. An estimate, not FEA or certification; the
//! explicit-support truss workflow (`truss_solve`) is the FEA path.
use super::report_util::*;
use super::strength_material::*;
use super::strength_report;
use super::weak_spots::{self, Graph, LoadCase, WeakInput};
use super::*;
use std::f64::consts::PI;
use value_codec::json;

pub(super) struct Section {
    pub area: f64,
    pub inertia: f64,
    pub modulus: f64,
    pub label: String,
    pub eccentricity: f64,
}

pub(super) fn section(v: &Value, rib: f64) -> Result<Section> {
    let empty = json!({});
    let v = if v.is_null() { &empty } else { v };
    let a = opt_num(v, "a", rib)?.max(1e-6);
    let ecc = opt_num(v, "eccentricity", 0.)?.max(0.);
    let f = |x: f64| rnd(x, 2);
    let (area, inertia, modulus, label) = match v["kind"].as_str().unwrap_or("circle") {
        "circle" => {
            let r = a / 2.;
            let i = PI * r.powi(4) / 4.;
            (PI * r * r, i, i / r, format!("⌀{}", f(a)))
        }
        "square" => {
            let i = a.powi(4) / 12.;
            (a * a, i, i / (a / 2.), format!("{}×{}", f(a), f(a)))
        }
        "tube" => {
            let ro = a / 2.;
            let ri = (opt_num(v, "b", a * 0.6)? / 2.).clamp(0., ro * 0.95);
            let i = PI / 4. * (ro.powi(4) - ri.powi(4));
            (PI * (ro * ro - ri * ri), i, i / ro, format!("⌀{}/⌀{}", f(a), f(ri * 2.)))
        }
        "rectangle" => {
            let b = opt_num(v, "b", a)?.max(1e-6);
            let i = b * a.powi(3) / 12.;
            (a * b, i, i / (a / 2.), format!("{}×{}", f(a), f(b)))
        }
        _ => return Err(input("Strut section must be circle, square, rectangle or tube.")),
    };
    Ok(Section { area, inertia, modulus, label, eccentricity: ecc })
}

/// Geometric relative density estimate before meshing.
pub(super) fn estimate_relative_density(o: &LatticeParams, size: [f64; 3]) -> f64 {
    let [sx, sy, sz] = size;
    let vol = (sx * sy * sz).max(1e-12);
    let cell = o.cell.max(o.rib * 2.);
    if o.spatial() {
        let r = o.rib / 2.;
        let struts = match o.pattern.as_str() {
            "octet" => 12.,
            "bcc" => 8.,
            "bone" => 7.,
            _ if o.diagonals => 4.,
            _ => 3.,
        };
        let len = cell * match o.pattern.as_str() { "bcc" => 3f64.sqrt(), "octet" => 2f64.sqrt(), _ => 1. };
        let mut rho = (struts * PI * r * r * len / cell.powi(3)).min(0.95);
        if o.wall_depth > 0. {
            let b = o.wall_depth;
            let shell = (vol - (sx - 2. * b).max(0.) * (sy - 2. * b).max(0.) * (sz - 2. * b).max(0.)).max(0.);
            rho = if o.keep_core {
                (rho * shell / vol + (vol - shell) / vol).min(0.98)
            } else {
                (rho * shell / vol).max(0.02).min(0.95)
            };
        }
        if o.skin > 0. {
            rho = (rho + 2. * (sx * sy + sx * sz + sy * sz) * o.skin / vol).min(0.98);
        }
        return rnd(rho.max(0.02), 4);
    }
    let open = match o.pattern.as_str() {
        "isogrid" | "triangles" => 0.72,
        "honeycomb" => 0.78,
        "web" => 0.7,
        _ => 0.65,
    };
    let (u, v) = ((o.axis + 1) % 3, (o.axis + 2) % 3);
    let span_u = (size[u] - 2. * o.rim).max(1e-6);
    let span_v = (size[v] - 2. * o.rim).max(1e-6);
    let solid = 1. - open * (1. - o.rib / cell).powi(2);
    let skins = (o.bottom + o.top) / size[o.axis].max(1e-6);
    let rim = 1. - span_u * span_v / (size[u] * size[v]).max(1e-12);
    rnd((solid * (1. - rim) * (1. - skins) + skins + rim * 0.5).clamp(0.05, 0.98), 4)
}

/// Local plate buckling stress of a cell wall, MPa.
pub(super) fn panel_buckling(e: f64, wall: f64, cell: f64) -> f64 {
    let nu: f64 = 0.3;
    let t = wall.max(1e-6);
    let b = cell.max(t);
    4. * PI * PI * e / (12. * (1. - nu * nu)) * (t / b).powi(2)
}

fn ashby_delta(scale: f64, n: f64) -> (f64, f64) {
    let r = scale.max(1e-6);
    (((r.powf(n) - 1.) * 100.).round(), ((r - 1.) * 100.).round())
}

pub(super) fn advice(o: &LatticeParams, util: f64, buckling: f64, service: &Service, n: f64) -> Vec<Value> {
    let mut tips = Vec::new();
    fn push(tips: &mut Vec<Value>, ru: String, en: String, (ds, dm): (f64, f64)) {
        tips.push(json!({"ru": ru, "en": en, "deltaStiffnessPct": ds, "deltaMassPct": dm}));
    }
    let strut_scale = if matches!(o.pattern.as_str(), "grid" | "web" | "honeycomb") { 1 } else { 2 };
    if buckling > 0.7 || util > 1. {
        push(&mut tips, format!("Увеличить диаметр стержня {:.1} → {:.1} mm", o.rib, o.rib * 1.25),
            format!("Increase strut diameter {:.1} → {:.1} mm", o.rib, o.rib * 1.25),
            ashby_delta(1.25f64.powi(strut_scale), n));
        push(&mut tips, format!("Уменьшить ячейку {:.1} → {:.1} mm", o.cell, o.cell * 0.85),
            format!("Reduce cell {:.1} → {:.1} mm", o.cell, o.cell * 0.85),
            ashby_delta((1. / 0.85f64).powi(strut_scale), n));
    }
    let diagonal = o.diagonals || matches!(o.pattern.as_str(), "octet" | "isogrid");
    if !diagonal && matches!(o.pattern.as_str(), "spatial" | "grid" | "bcc") {
        push(&mut tips, "Добавить диагонали (stretch-dominated) — рост E* при той же ρ*".into(),
            "Add diagonals (stretch-dominated) — higher E* at same ρ*".into(),
            ((25. + 10. * (n - 1.).max(0.)).round(), 12.));
    }
    if o.cell - o.rib > service.max_bridge_mm {
        push(&mut tips, format!("Мост {:.1} mm > лимита {} mm", o.cell - o.rib, service.max_bridge_mm),
            format!("Bridge {:.1} mm > limit {} mm", o.cell - o.rib, service.max_bridge_mm), (0., 0.));
    }
    if o.rib < service.min_wall_mm {
        push(&mut tips, format!("Стержень {} mm < min wall {} mm", o.rib, service.min_wall_mm),
            format!("Strut {} mm < min wall {} mm", o.rib, service.min_wall_mm), (0., 0.));
    }
    if o.wall_depth > 0. && o.wall_depth < o.rib * 2. {
        push(&mut tips, "Углубить скелетную стенку ≥ 2·диаметра стержня".into(),
            "Deepen skeletal wall to ≥ 2× strut diameter".into(), ashby_delta(1.15, n));
    }
    if tips.is_empty() {
        push(&mut tips, "Запасы приемлемы; можно слегка увеличить ячейку для экономии массы".into(),
            "Margins OK; slightly larger cell can save mass".into(), ashby_delta(0.88, n));
    }
    tips
}

/// `cad_lattice_strength`: `{mesh, options, material, load:{case, forceN, safety,
/// direction?}, volumes?, service?, section?}` → strength report with weak spots.
pub fn strength(v: Value) -> Result<Value> {
    let mesh: Mesh = field(&v, "mesh")?;
    mesh.validate()?;
    let options = v["options"].clone();
    let o = LatticeParams::parse(&options)?;
    let material = Material::parse(&v["material"])?;
    let load = &v["load"];
    let case = LoadCase::parse(load["case"].as_str())?;
    let force_n = required_lattice_option(load, "forceN")?;
    let safety = required_lattice_option(load, "safety")?;
    if force_n <= 0. || safety < 1. {
        return Err(input("Check force (>0 N) and safety factor (>=1)."));
    }
    let direction = match load.get("direction") {
        None | Some(Value::Null) => [0., 0., -1.],
        Some(d) => {
            let d: [f64; 3] = value_codec::from_value(d.clone()).map_err(|_| input("Load direction must be XYZ."))?;
            if !d.iter().all(|x| x.is_finite()) || d.iter().all(|x| *x == 0.) {
                return Err(input("Load direction must be a finite non-zero vector."));
            }
            d
        }
    };
    let service = Service::parse(&v["service"], o.axis)?;
    let sec = section(&v["section"], o.rib)?;
    let (min, max) = mesh_bounds(&mesh)?;
    let graph = if o.spatial() {
        let g = graph(json!({"mesh": v["mesh"].clone(), "options": options.clone()}))?;
        let nodes: Vec<[f64; 3]> = field(&g, "nodes")?;
        let edges: Vec<[usize; 2]> = field(&g, "edges")?;
        Some(Graph::new(nodes, edges)?)
    } else {
        None
    };
    let volumes = match &v["volumes"] {
        Value::Null => None,
        vol => {
            let (b, a) = (required_lattice_option(vol, "beforeMm3")?, required_lattice_option(vol, "afterMm3")?);
            (b > 0. && a >= 0.).then_some((b, a))
        }
    };
    let input = strength_report::Input {
        o: &o, min, max, graph: graph.as_ref(), material: &material, case,
        force_n, safety, direction, service: &service, section: &sec, volumes,
    };
    let mut report = strength_report::analyze(&input);
    let spots = weak_spots::find(&WeakInput {
        o: &o, options: &options, min, max, graph: graph.as_ref(), material: &material, load: case, force_n,
    }, 12);
    strength_report::finish(&mut report, spots);
    Ok(report)
}
