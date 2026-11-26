//! STEP entity allocation and spline encoding.
use super::*;

pub(crate) struct Writer {
    next: usize,
    shared_points: Option<BTreeMap<Vec<u64>, usize>>,
    pub(crate) rows: BTreeMap<usize, String>,
}
impl Writer {
    pub(crate) fn new() -> Self {
        Self {
            next: 1,
            shared_points: None,
            rows: BTreeMap::new(),
        }
    }
    pub(super) fn enable_point_sharing(&mut self) {
        self.shared_points = Some(BTreeMap::new());
    }

    fn reserve(&mut self) -> usize {
        let id = self.next;
        self.next += 1;
        id
    }
    fn set(&mut self, id: usize, body: String) {
        self.rows.insert(id, body);
    }
    pub(crate) fn emit(&mut self, body: String) -> usize {
        let id = self.reserve();
        self.set(id, body);
        id
    }
    pub(crate) fn point(&mut self, p: &[f64]) -> usize {
        let Some(points) = &self.shared_points else {
            return self.emit(format!("CARTESIAN_POINT('',({}))", floats(p)));
        };
        let key: Vec<_> = p.iter().map(|value| value.to_bits()).collect();
        if let Some(&id) = points.get(&key) {
            return id;
        }
        let id = self.emit(format!("CARTESIAN_POINT('',({}))", floats(p)));
        if let Some(points) = &mut self.shared_points {
            points.insert(key, id);
        }
        id
    }
}
pub(super) fn floats(v: &[f64]) -> String {
    v.iter()
        .map(|x| format!("{x:.17e}").replace('e', "E"))
        .collect::<Vec<_>>()
        .join(",")
}
pub(crate) fn enforce_output_limit(text: &str) -> Result<()> {
    if text.len() > MAX_OUTPUT_BYTES {
        return Err(refuse("STEP output exceeds 16 MiB"));
    }
    Ok(())
}
pub(super) fn quadratic_patch_knots(segments: usize) -> Vec<f64> {
    let mut knots = vec![0.; 3];
    for knot in 1..segments {
        knots.extend([knot as f64, knot as f64])
    }
    knots.extend([segments as f64; 3]);
    knots
}


// Face counts and degrees do not establish a periodic carrier: partial
// revolutions can have the same shape of patch records as a complete band.



pub(super) fn constant_boundary(surface: &Surface, column: usize) -> bool {
    surface
        .control_points
        .iter()
        .all(|row| row[column] == surface.control_points[0][column])
}


pub(crate) fn canonical_face_senses(model: &Model) -> Result<Model> {
    let mut result = model.clone();
    let reversed = result
        .shells
        .iter()
        .flat_map(|shell| &shell.faces)
        .filter(|usage| usage.reversed)
        .map(|usage| usage.face)
        .collect::<BTreeSet<_>>();
    for face_index in reversed {
        let face = &mut result.0.faces[face_index];
        let sum = face.surface.knots_u[face.surface.degree_u]
            + face.surface.knots_u[face.surface.knots_u.len() - face.surface.degree_u - 1];
        face.surface.control_points.reverse();
        face.surface.weights.reverse();
        face.surface.knots_u.reverse();
        for knot in &mut face.surface.knots_u {
            *knot = sum - *knot
        }
        let mut loops = vec![face.outer];
        loops.extend(face.holes.iter().copied());
        for loop_index in loops {
            for coedge in &mut result.0.loops[loop_index].coedges {
                for point in &mut coedge.pcurve.control_points {
                    point[0] = sum - point[0]
                }
                coedge.pcurve = coedge.pcurve.reverse()?;
                coedge.reversed = !coedge.reversed;
            }
            result.0.loops[loop_index].coedges.reverse();
        }
    }
    for shell in &mut result.0.shells {
        for usage in &mut shell.faces {
            usage.reversed = false
        }
    }
    result.validate()?;
    Ok(result)
}
pub(crate) fn refs_text(v: &[usize]) -> String {
    v.iter()
        .map(|x| format!("#{x}"))
        .collect::<Vec<_>>()
        .join(",")
}
fn knot_form(knots: &[f64]) -> (Vec<usize>, Vec<f64>) {
    let mut m = Vec::new();
    let mut k = Vec::new();
    for &x in knots {
        if k.last()
            .is_some_and(|v: &f64| (*v - x).abs() <= f64::EPSILON)
        {
            *m.last_mut().expect("m grows in lockstep with k, which is non-empty here") += 1;
        } else {
            k.push(x);
            m.push(1);
        }
    }
    (m, k)
}
pub(super) fn emit_curve(w: &mut Writer, c: &Curve) -> usize {
    let cps: Vec<_> = c.control_points.iter().map(|p| w.point(p)).collect();
    let (m, k) = knot_form(&c.knots);
    w.emit(format!("B_SPLINE_CURVE_WITH_KNOTS('',{},({}),.UNSPECIFIED.,.F.,.{closed}.,.F.,({}),({}),({}),.UNSPECIFIED.)",
        c.degree, refs_text(&cps), floats(&c.weights), m.iter().map(ToString::to_string).collect::<Vec<_>>().join(","), floats(&k),
        closed = if c.periodic { "T" } else { "F" }))
}
pub(super) fn emit_surface(w: &mut Writer, s: &Surface) -> usize {
    let grid = s
        .control_points
        .iter()
        .map(|row| {
            let ids: Vec<_> = row.iter().map(|p| w.point(p)).collect();
            format!("({})", refs_text(&ids))
        })
        .collect::<Vec<_>>()
        .join(",");
    let (mu, ku) = knot_form(&s.knots_u);
    let (mv, kv) = knot_form(&s.knots_v);
    w.emit(format!("B_SPLINE_SURFACE_WITH_KNOTS('',{},{},({grid}),.UNSPECIFIED.,.F.,.{pu}.,.{pv}.,.F.,.F.,({}),({}),({}),({}),({}),.UNSPECIFIED.)",
        s.degree_u, s.degree_v, floats(&s.weights.iter().flatten().copied().collect::<Vec<_>>()),
        mu.iter().map(ToString::to_string).collect::<Vec<_>>().join(","),
        mv.iter().map(ToString::to_string).collect::<Vec<_>>().join(","), floats(&ku), floats(&kv),
        pu=if s.periodic_u {"T"} else {"F"}, pv=if s.periodic_v {"T"} else {"F"}))
}
fn all_unit_weights(mut weights: impl Iterator<Item = f64>) -> bool {
    weights.all(|weight| (weight - 1.).abs() <= f64::EPSILON)
}
fn knot_form_exchange(knots: &[f64], exact: bool) -> (Vec<usize>, Vec<f64>) {
    if !exact {
        return knot_form(knots);
    }
    let mut counts = Vec::new();
    let mut values = Vec::new();
    for &knot in knots {
        if values.last() == Some(&knot) {
            *counts.last_mut().expect("counts grows in lockstep with values, which is non-empty here") += 1;
        } else {
            values.push(knot);
            counts.push(1);
        }
    }
    (counts, values)
}
pub(crate) fn emit_curve_v5(w: &mut Writer, c: &Curve) -> usize {
    emit_curve_exchange(w, c, false)
}
pub(crate) fn emit_curve_source(w: &mut Writer, c: &Curve) -> usize {
    emit_curve_exchange(w, c, true)
}
fn emit_curve_exchange(w: &mut Writer, c: &Curve, exact: bool) -> usize {
    let cps: Vec<_> = c.control_points.iter().map(|p| w.point(p)).collect();
    let (m, k) = knot_form_exchange(&c.knots, exact);
    let mult = m
        .iter()
        .map(ToString::to_string)
        .collect::<Vec<_>>()
        .join(",");
    let closed = if c.periodic { "T" } else { "F" };
    let unit = if exact {
        c.weights.iter().all(|&v| v == 1.)
    } else {
        all_unit_weights(c.weights.iter().copied())
    };
    if unit {
        w.emit(format!("B_SPLINE_CURVE_WITH_KNOTS('',{},({}),.UNSPECIFIED.,.{closed}.,.F.,({mult}),({}),.UNSPECIFIED.)",
            c.degree,refs_text(&cps),floats(&k)))
    } else {
        w.emit(format!("(BOUNDED_CURVE()B_SPLINE_CURVE({},({}),.UNSPECIFIED.,.{closed}.,.F.)B_SPLINE_CURVE_WITH_KNOTS(({mult}),({}),.UNSPECIFIED.)CURVE()GEOMETRIC_REPRESENTATION_ITEM()RATIONAL_B_SPLINE_CURVE(({}))REPRESENTATION_ITEM(''))",
            c.degree,refs_text(&cps),floats(&k),floats(&c.weights)))
    }
}
pub(super) fn emit_surface_v5(w: &mut Writer, s: &Surface) -> usize {
    emit_surface_exchange(w, s, false)
}
pub(crate) fn emit_surface_source(w: &mut Writer, s: &Surface) -> usize {
    emit_surface_exchange(w, s, true)
}
fn emit_surface_exchange(w: &mut Writer, s: &Surface, exact: bool) -> usize {
    let grid = s
        .control_points
        .iter()
        .map(|row| {
            let ids: Vec<_> = row.iter().map(|p| w.point(p)).collect();
            format!("({})", refs_text(&ids))
        })
        .collect::<Vec<_>>()
        .join(",");
    let weights = s
        .weights
        .iter()
        .map(|row| format!("({})", floats(row)))
        .collect::<Vec<_>>()
        .join(",");
    let (mu, ku) = knot_form_exchange(&s.knots_u, exact);
    let (mv, kv) = knot_form_exchange(&s.knots_v, exact);
    let mu = mu
        .iter()
        .map(ToString::to_string)
        .collect::<Vec<_>>()
        .join(",");
    let mv = mv
        .iter()
        .map(ToString::to_string)
        .collect::<Vec<_>>()
        .join(",");
    let pu = if s.periodic_u { "T" } else { "F" };
    let pv = if s.periodic_v { "T" } else { "F" };
    let unit = if exact {
        s.weights.iter().flatten().all(|&v| v == 1.)
    } else {
        all_unit_weights(s.weights.iter().flatten().copied())
    };
    if unit {
        w.emit(format!("B_SPLINE_SURFACE_WITH_KNOTS('',{},{},({grid}),.UNSPECIFIED.,.{pu}.,.{pv}.,.F.,({mu}),({mv}),({}),({}),.UNSPECIFIED.)",
            s.degree_u,s.degree_v,floats(&ku),floats(&kv)))
    } else {
        w.emit(format!("(BOUNDED_SURFACE()B_SPLINE_SURFACE({},{},({grid}),.UNSPECIFIED.,.{pu}.,.{pv}.,.F.)B_SPLINE_SURFACE_WITH_KNOTS(({mu}),({mv}),({}),({}),.UNSPECIFIED.)GEOMETRIC_REPRESENTATION_ITEM()RATIONAL_B_SPLINE_SURFACE(({weights}))REPRESENTATION_ITEM('')SURFACE())",
            s.degree_u,s.degree_v,floats(&ku),floats(&kv)))
    }
}
