//! Complete source vertical-seam image and endpoint authority. Does not replace
//! legacy side pcurves or claim coupled-parameter identity.
use super::*;
use crate::profile_certificate::original_restriction::{self as exact, vertical};
#[derive(Clone, Debug)]
pub struct Binding {
    edge: usize,
    sides: [usize; 2],
    coedges: [[usize; 2]; 2],
    proof: vertical::Vertical,
}
impl Binding {
    pub fn edge(&self) -> usize {
        self.edge
    }
    pub fn sides(&self) -> [usize; 2] {
        self.sides
    }
    pub fn coedges(&self) -> [[usize; 2]; 2] {
        self.coedges
    }
    pub fn proof(&self) -> &vertical::Vertical {
        &self.proof
    }
}
#[derive(Clone, Debug)]
pub struct VerticalSeams {
    lower: lower_rims::LowerRims,
    bindings: Vec<Binding>,
}
struct Use {
    side: usize,
    coedge: [usize; 2],
    oriented: bool,
    endpoints: [[ExactRatio; 3]; 2],
}
fn derive(lower: &lower_rims::LowerRims) -> Result<Vec<Binding>> {
    lower.reverify()?;
    let supports = lower.supports();
    let model = &supports.source;
    let mut uses = BTreeMap::<usize, Vec<Use>>::new();
    for data in &supports.faces {
        let face = &model.faces[data.face];
        let face_uses = model.shells[0]
            .faces
            .iter()
            .filter(|u| u.face == data.face)
            .collect::<Vec<_>>();
        if face_uses.len() != 1 {
            return Err(error("Vertical seam side has no unique shell use"));
        }
        let profile = exact::restrict_binary64(&data.profile, data.profile.domain())?;
        let d = data.rectangle[data.axis];
        let r = Q::new(
            data.ratio[0]
                .parse()
                .map_err(|_| error("Invalid vertical ratio"))?,
            data.ratio[1]
                .parse()
                .map_err(|_| error("Invalid vertical ratio"))?,
        );
        for (ci, c) in model.loops[face.outer].coedges.iter().enumerate() {
            let p = &c.pcurve.control_points;
            if p[0][1 - data.axis] != p[1][1 - data.axis] {
                continue;
            }
            let xy = profile.evaluate_natural(ratio_value(&q(p[0][1 - data.axis])?))?;
            let mut endpoints = Vec::new();
            for uv in p {
                let v = (q(uv[data.axis])? - q(d[0])?) / (q(d[1])? - q(d[0])?);
                let one_minus_v = q(1.)? - &v;
                let z = (&one_minus_v * q(data.vertical_z[0])? + &v * &r * q(data.vertical_z[1])?)
                    / (&one_minus_v + &v * &r);
                endpoints.push([xy[0].clone(), xy[1].clone(), ratio_value(&z)]);
            }
            let mut endpoints: [[ExactRatio; 3]; 2] = endpoints
                .try_into()
                .map_err(|_| error("Unexpected native vertical pcurve degree"))?;
            if c.reversed {
                endpoints.reverse();
            }
            uses.entry(c.edge).or_default().push(Use {
                side: data.face,
                coedge: [face.outer, ci],
                oriented: c.reversed ^ face_uses[0].reversed,
                endpoints,
            });
        }
    }
    let mut bindings = Vec::new();
    for (edge, owners) in uses {
        if owners.len() != 2
            || owners[0].side == owners[1].side
            || owners[0].oriented == owners[1].oriented
            || owners[0].endpoints != owners[1].endpoints
        {
            return Err(error(
                "Native vertical seam has incompatible side owners or endpoints",
            ));
        }
        let e = &model.edges[edge];
        let endpoints = &owners[0].endpoints;
        let decode = |v: &ExactRatio| -> Result<Q> {
            Ok(Q::new(
                v[0].parse().map_err(|_| error("Invalid seam numerator"))?,
                v[1].parse()
                    .map_err(|_| error("Invalid seam denominator"))?,
            ))
        };
        let z = [decode(&endpoints[0][2])?, decode(&endpoints[1][2])?];
        let height = if z[0] < z[1] {
            [ratio_value(&z[0]), ratio_value(&z[1])]
        } else {
            [ratio_value(&z[1]), ratio_value(&z[0])]
        };
        let proof = vertical::qualify(
            &e.curve,
            [endpoints[0][0].clone(), endpoints[0][1].clone()],
            height,
        )?;
        if proof.endpoints() != endpoints {
            return Err(error("Native vertical clock direction differs from side"));
        }
        for (i, point) in endpoints.iter().enumerate() {
            for (k, value) in point.iter().enumerate() {
                if q(model.vertices[e.vertices[i]].point[k])? != decode(value)? {
                    return Err(error(
                        "Native vertical vertex differs from exact edge endpoint",
                    ));
                }
            }
        }
        bindings.push(Binding {
            edge,
            sides: [owners[0].side, owners[1].side],
            coedges: [owners[0].coedge, owners[1].coedge],
            proof,
        });
    }
    let mut all = BTreeSet::new();
    for edge in supports
        .bindings
        .iter()
        .map(|b| b.edge)
        .chain(lower.bindings().iter().map(|b| b.edge()))
        .chain(bindings.iter().map(|b| b.edge))
    {
        if !all.insert(edge) {
            return Err(error("Original horizontal/vertical edge census overlaps"));
        }
    }
    if all != (0..model.edges.len()).collect() {
        return Err(error("Original edge census is incomplete"));
    }
    Ok(bindings)
}
impl lower_rims::LowerRims {
    pub fn qualify_vertical_seams(&self) -> Result<VerticalSeams> {
        Ok(VerticalSeams {
            lower: self.clone(),
            bindings: derive(self)?,
        })
    }
}
impl VerticalSeams {
    pub fn lower_rims(&self) -> &lower_rims::LowerRims {
        &self.lower
    }
    pub fn bindings(&self) -> &[Binding] {
        &self.bindings
    }
    pub fn reverify(&self) -> Result<()> {
        let expected = derive(&self.lower)?;
        if expected.len() != self.bindings.len() {
            return Err(error("Vertical seam census changed"));
        }
        for (a, b) in self.bindings.iter().zip(expected) {
            a.proof.reverify()?;
            if a.edge != b.edge
                || a.sides != b.sides
                || a.coedges != b.coedges
                || a.proof.source() != b.proof.source()
                || a.proof.endpoints() != b.proof.endpoints()
            {
                return Err(error("Vertical seam original source replay changed"));
            }
        }
        Ok(())
    }
}
