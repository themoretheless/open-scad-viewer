//! Exact Bezier boundary geometry on plane and ruled-extrusion supports.
use super::*;
use crate::profile_certificate::arithmetic;
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Curve3 {
    pub(super) net: Vec<[Q; 4]>,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Pcurve {
    pub(super) net: Vec<[Q; 3]>,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) enum Kind {
    Cap {
        z: Q,
        bounds: [[Q; 2]; 2],
    },
    Wall {
        profile: Vec<[Q; 3]>,
        height: [Q; 2],
    },
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Surface {
    pub(super) kind: Kind,
}
impl Curve3 {
    pub fn homogeneous_controls(&self) -> Vec<[ExactRatio; 4]> {
        self.net.iter().map(|h| h.each_ref().map(ratio)).collect()
    }
}
impl Pcurve {
    pub fn homogeneous_controls(&self) -> Vec<[ExactRatio; 3]> {
        self.net.iter().map(|h| h.each_ref().map(ratio)).collect()
    }
}
impl Surface {
    fn bounds(&self) -> [[Q; 2]; 2] {
        match &self.kind {
            Kind::Cap { bounds, .. } => bounds.clone(),
            Kind::Wall { height, .. } => [
                [Q::zero(), height[0].clone()],
                [Q::one(), height[1].clone()],
            ],
        }
    }
    pub fn parameter_bounds(&self) -> [[ExactRatio; 2]; 2] {
        self.bounds().map(|p| p.each_ref().map(ratio))
    }
    pub fn homogeneous_controls(&self) -> Vec<Vec<[ExactRatio; 4]>> {
        let net = match &self.kind {
            Kind::Cap { z, bounds } => [0, 1]
                .map(|u| {
                    [0, 1]
                        .map(|v| {
                            [
                                bounds[u][0].clone(),
                                bounds[v][1].clone(),
                                z.clone(),
                                Q::one(),
                            ]
                        })
                        .to_vec()
                })
                .to_vec(),
            Kind::Wall { profile, height } => profile
                .iter()
                .map(|h| {
                    height
                        .iter()
                        .map(|z| [h[0].clone(), h[1].clone(), z * &h[2], h[2].clone()])
                        .collect()
                })
                .collect(),
        };
        net.iter()
            .map(|r| r.iter().map(|h| h.each_ref().map(ratio)).collect())
            .collect()
    }
    fn pullback(&self, p: &Pcurve) -> Result<Curve3> {
        if p.net.is_empty() || p.net.iter().any(|h| h[2] <= Q::zero()) {
            return Err(refused("Invalid exact pcurve weights"));
        }
        let bounds = self.bounds();
        for h in &p.net {
            for a in 0..2 {
                if h[a] < &bounds[0][a] * &h[2] || h[a] > &bounds[1][a] * &h[2] {
                    return Err(refused("Exact pcurve leaves its support chart"));
                }
            }
        }
        match &self.kind {
            Kind::Cap { z, .. } => Ok(Curve3 {
                net: p
                    .net
                    .iter()
                    .map(|h| [h[0].clone(), h[1].clone(), z * &h[2], h[2].clone()])
                    .collect(),
            }),
            Kind::Wall { profile, .. } => {
                if p.net.len() != 2 || p.net.iter().any(|h| h[2] != Q::one()) {
                    return Err(refused(
                        "Exact wall requires its authored affine boundary pcurve",
                    ));
                }
                let a = &p.net[0];
                let b = &p.net[1];
                if a[1] == b[1]
                    && ((a[0].is_zero() && b[0].is_one()) || (a[0].is_one() && b[0].is_zero()))
                {
                    let mut net = profile
                        .iter()
                        .map(|h| [h[0].clone(), h[1].clone(), &a[1] * &h[2], h[2].clone()])
                        .collect::<Vec<_>>();
                    if a[0] > b[0] {
                        net.reverse();
                    }
                    Ok(Curve3 { net })
                } else if a[0] == b[0] && (a[0].is_zero() || a[0].is_one()) {
                    let h = if a[0].is_zero() {
                        &profile[0]
                    } else {
                        profile.last().unwrap()
                    };
                    Ok(Curve3 {
                        net: p
                            .net
                            .iter()
                            .map(|v| [&h[0] / &h[2], &h[1] / &h[2], v[1].clone(), Q::one()])
                            .collect(),
                    })
                } else {
                    Err(refused("Exact wall pcurve is not an authored boundary"))
                }
            }
        }
    }
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Agreement {
    pub edges: usize,
    pub coedges: usize,
}
pub fn verify_geometry(model: &Topology) -> Result<Agreement> {
    for edge in &model.edges {
        if edge.curve.net.len() < 2 || edge.curve.net.iter().any(|h| h[3] <= Q::zero()) {
            return Err(refused("Invalid exact edge weights"));
        }
        for end in 0..2 {
            let h = if end == 0 {
                &edge.curve.net[0]
            } else {
                edge.curve.net.last().unwrap()
            };
            let vertex = model
                .vertices
                .get(edge.vertices[end])
                .ok_or_else(|| refused("Unknown exact edge vertex"))?;
            if (0..3).any(|a| h[a] != &vertex.point[a] * &h[3]) {
                return Err(refused("Exact edge endpoint disagrees with its vertex"));
            }
        }
    }
    let mut coedges = 0;
    for face in &model.faces {
        for w in std::iter::once(face.outer).chain(face.holes.iter().copied()) {
            let wire = model
                .loops
                .get(w)
                .ok_or_else(|| refused("Unknown exact face loop"))?;
            for c in &wire.coedges {
                if c.trim().is_some() {
                    return Err(refused("Unexpected trim on exact authored coedge"));
                }
                let pullback = face.surface.pullback(&c.pcurve)?;
                let mut actual = model
                    .edges
                    .get(c.edge)
                    .ok_or_else(|| refused("Unknown exact face edge"))?
                    .curve
                    .net
                    .clone();
                if c.reversed {
                    actual.reverse();
                }
                for axis in 0..3 {
                    let project = |net: &[[Q; 4]]| {
                        net.iter()
                            .map(|h| [h[axis].clone(), Q::zero(), h[3].clone()])
                            .collect::<Vec<_>>()
                    };
                    if !arithmetic::projectively_equal(&project(&actual), &project(&pullback.net))?
                    {
                        return Err(refused(
                            "Exact edge and surface-pcurve composition disagree",
                        ));
                    }
                }
                coedges += 1;
            }
        }
    }
    Ok(Agreement {
        edges: model.edges.len(),
        coedges,
    })
}
