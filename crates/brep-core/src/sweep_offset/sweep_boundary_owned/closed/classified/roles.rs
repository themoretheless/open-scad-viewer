//! Source-bound classes for every actual coedge, including the periodic seam.
use super::*;
use crate::sweep_smoothness::{self, Axis, complete::classified::Class};
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Role {
    Profile { join: usize, layer: usize },
    Station { station: usize, span: usize },
}
pub struct Report {
    pub certified: bool,
    pub classes: Vec<Class>,
    pub roles: Vec<Option<Role>>,
    pub steps: usize,
    pub uncertain_edge: Option<usize>,
    pub reason: &'static str,
}
#[derive(Clone, Copy)]
struct Port {
    span: usize,
    layer: usize,
    axis: usize,
    upper: bool,
}
fn step(out: &mut Report, max: usize) -> bool {
    if out.steps == max {
        out.reason = "classified-closed-role-work-limit";
        false
    } else {
        out.steps += 1;
        true
    }
}
pub(super) fn inspect(
    model: &Model,
    profiles: &[Vec<Curve>],
    layers: usize,
    binding: &smooth::BindingReport,
    classes: &[OriginalProfileClass],
    sharp: &[usize],
    station_order: usize,
    max_steps: usize,
) -> Result<Report> {
    let spans = profiles.iter().map(Vec::len).sum::<usize>();
    if layers == 0
        || classes.len() != spans
        || !binding.certified
        || !matches!(station_order, 1 | 2)
        || max_steps > 1000000
        || profiles.iter().any(Vec::is_empty)
        || sharp.iter().any(|s| *s > layers)
    {
        return Err(fail(
            "Use complete owned span classes and bounded periodic roles",
        ));
    }
    let mut out = Report {
        certified: false,
        classes: vec![Class::C0; model.edges.len()],
        roles: vec![None; model.edges.len()],
        steps: 0,
        uncertain_edge: None,
        reason: "classified-closed-native-roles-unproven",
    };
    let mut inverse = vec![usize::MAX; model.faces.len()];
    for slot in 0..spans * layers {
        if !step(&mut out, max_steps) {
            return Ok(out);
        }
        let Some(face) = binding.source_face(slot) else {
            return Ok(out);
        };
        if inverse[face] != usize::MAX {
            return Ok(out);
        }
        inverse[face] = slot;
    }
    if inverse.contains(&usize::MAX) {
        return Ok(out);
    }
    let mut rings = Vec::new();
    let mut offset = 0;
    for wire in profiles {
        for _ in wire {
            if !step(&mut out, max_steps) {
                return Ok(out);
            }
            rings.push((offset, offset + wire.len()));
        }
        offset += wire.len();
    }
    let mut uses = vec![Vec::new(); model.edges.len()];
    for (face, f) in model.faces.iter().enumerate() {
        if !f.holes.is_empty() {
            return Ok(out);
        }
        for c in &model.loops[f.outer].coedges {
            if !step(&mut out, max_steps) {
                return Ok(out);
            }
            let Some(label) = sweep_smoothness::boundary(&f.surface, &c.pcurve, Axis::Profile)
                .or_else(|| sweep_smoothness::boundary(&f.surface, &c.pcurve, Axis::Station))
            else {
                return Ok(out);
            };
            let axis = usize::from(label.starts_with('v'));
            let upper = label.ends_with("Max");
            let Some((axis, upper)) = binding.source_side(face, axis, upper) else {
                return Ok(out);
            };
            let slot = inverse[face];
            uses[c.edge].push(Port {
                span: slot % spans,
                layer: slot / spans,
                axis,
                upper,
            });
        }
    }
    for (edge, ports) in uses.iter().enumerate() {
        out.uncertain_edge = Some(edge);
        if !step(&mut out, max_steps) {
            return Ok(out);
        }
        let [a, b] = ports.as_slice() else {
            return Ok(out);
        };
        if a.axis != b.axis || a.upper == b.upper {
            return Ok(out);
        }
        let (left, right) = if a.upper { (a, b) } else { (b, a) };
        let (class, role) = if left.axis == 0 {
            let (lo, hi) = rings[left.span];
            let next = if left.span + 1 == hi {
                lo
            } else {
                left.span + 1
            };
            if right.span != next || right.layer != left.layer || rings[right.span] != (lo, hi) {
                return Ok(out);
            }
            let class = match classes[left.span] {
                OriginalProfileClass::C0 => Class::C0,
                OriginalProfileClass::G1 => Class::G1,
                OriginalProfileClass::G2 => Class::G2,
            };
            (
                class,
                Role::Profile {
                    join: left.span,
                    layer: left.layer,
                },
            )
        } else {
            let next = (left.layer + 1) % layers;
            if right.layer != next || right.span != left.span {
                return Ok(out);
            }
            let sharp = sharp.contains(&next) || (next == 0 && sharp.contains(&layers));
            let class = if sharp {
                Class::C0
            } else if station_order == 2 {
                Class::G2
            } else {
                Class::G1
            };
            (
                class,
                Role::Station {
                    station: next,
                    span: left.span,
                },
            )
        };
        out.classes[edge] = class;
        out.roles[edge] = Some(role);
    }
    out.uncertain_edge = None;
    out.certified = true;
    out.reason = "complete-source-bound-periodic-edge-roles-qualified";
    Ok(out)
}
