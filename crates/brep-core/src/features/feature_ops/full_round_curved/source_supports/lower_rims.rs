//! Exact image binding of every native lower-cap rim to its original side.
//! This completes horizontal rim coverage; vertical seams and solid admission
//! remain separate obligations.
use super::*;
#[derive(Clone, Debug, PartialEq)]
pub struct Binding {
    edge: usize,
    coedge: [usize; 2],
    side: usize,
    window: [f64; 2],
    proof: CurveProof,
}
impl Binding {
    pub fn edge(&self) -> usize {
        self.edge
    }
    pub fn coedge(&self) -> [usize; 2] {
        self.coedge
    }
    pub fn side(&self) -> usize {
        self.side
    }
    pub fn profile_window(&self) -> [f64; 2] {
        self.window
    }
}
#[derive(Clone, Debug)]
pub struct LowerRims {
    supports: SourceSupports,
    bindings: Vec<Binding>,
}
fn derive(supports: &SourceSupports) -> Result<Vec<Binding>> {
    supports.reverify()?;
    let model = &supports.source;
    let low = supports.height[0];
    let caps = model
        .faces
        .iter()
        .enumerate()
        .filter(|(_, f)| {
            f.surface
                .control_points
                .iter()
                .flatten()
                .all(|p| p[2] == low)
        })
        .collect::<Vec<_>>();
    if caps.len() != 1 {
        return Err(error("Lower rim needs one original bottom cap"));
    }
    let (cap_id, cap) = caps[0];
    let mut bindings = Vec::new();
    let mut used = BTreeMap::<usize, BTreeSet<usize>>::new();
    for w in std::iter::once(cap.outer).chain(cap.holes.iter().copied()) {
        for (ci, c) in model.loops[w].coedges.iter().enumerate() {
            if c.trim().is_some_and(|t| {
                t.curve_parameter != model.edges[c.edge].curve.domain()
                    || t.pcurve_parameter != c.pcurve.domain()
                    || t.periodic_lift != [[0; 2]; 2]
            }) {
                return Err(error("Lower cap has partial or lifted native coedge"));
            }
            let owners = (0..model.faces.len())
                .filter(|&f| f != cap_id && contains(model, f, c.edge))
                .collect::<Vec<_>>();
            if owners.len() != 1 {
                return Err(error("Lower rim needs one unique original side"));
            }
            let side = owners[0];
            let data = supports
                .faces
                .iter()
                .find(|f| f.face == side)
                .ok_or_else(|| error("Lower rim side is absent from whole-support census"))?;
            let coedges = model.loops[model.faces[side].outer]
                .coedges
                .iter()
                .filter(|s| s.edge == c.edge)
                .collect::<Vec<_>>();
            if coedges.len() != 1 {
                return Err(error("Lower rim repeated on original side"));
            }
            let s = coedges[0];
            let a = &s.pcurve.control_points[0];
            let b = &s.pcurve.control_points[1];
            let bottom =
                data.rectangle[data.axis][usize::from(data.top == data.rectangle[data.axis][0])];
            if a[data.axis] != bottom || b[data.axis] != bottom {
                return Err(error(
                    "Lower rim pcurve is not on the native bottom isocurve",
                ));
            }
            let world = raw_profiles::oriented(model, c.edge, c.reversed, low)?;
            let mut window = [a[1 - data.axis], b[1 - data.axis]];
            if s.reversed ^ c.reversed {
                window.reverse();
            }
            let proof = curve_proof(&world, &data.profile, window)?;
            let native = crate::profile_certificate::original_restriction::restrict_binary64(
                &data.profile,
                window,
            )?;
            let rim = crate::profile_certificate::original_restriction::restrict_binary64(
                &world,
                world.domain(),
            )?;
            for (native_t, rim_t) in native
                .parameter_range()
                .into_iter()
                .zip(rim.parameter_range())
            {
                if native.evaluate_natural(native_t)? != rim.evaluate_natural(rim_t)? {
                    return Err(error(
                        "Lower native profile has mismatched directed rim endpoints",
                    ));
                }
            }
            if !used.entry(side).or_default().insert(c.edge) {
                return Err(error("Duplicated lower rim"));
            }
            bindings.push(Binding {
                edge: c.edge,
                coedge: [w, ci],
                side,
                window,
                proof,
            });
        }
    }
    for data in &supports.faces {
        let expected = model.loops[model.faces[data.face].outer]
            .coedges
            .iter()
            .filter(|c| {
                let p = &c.pcurve.control_points;
                p[0][data.axis] == p[1][data.axis] && p[0][data.axis] != data.top
            })
            .map(|c| c.edge)
            .collect::<BTreeSet<_>>();
        if expected.is_empty() || used.get(&data.face) != Some(&expected) {
            return Err(error(
                "Lower cap does not cover the entire native side bottom",
            ));
        }
    }
    Ok(bindings)
}
impl SourceSupports {
    pub fn qualify_lower_rims(&self) -> Result<LowerRims> {
        Ok(LowerRims {
            supports: self.clone(),
            bindings: derive(self)?,
        })
    }
}
impl LowerRims {
    pub fn supports(&self) -> &SourceSupports {
        &self.supports
    }
    pub fn bindings(&self) -> &[Binding] {
        &self.bindings
    }
    pub fn reverify(&self) -> Result<()> {
        if derive(&self.supports)? != self.bindings {
            return Err(error("Original lower rim replay changed"));
        }
        Ok(())
    }
}
