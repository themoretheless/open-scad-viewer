//! Point location and exact 3D evaluation are available only through a checked
//! atlas handle. Algebraic interval midpoints never decide a stratum.
use super::*;
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum DomainLocation {
    Vertex(u32),
    Edge(u32),
    Face(u32),
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DomainEvaluation {
    pub parent: u32,
    pub cell: DomainLocation,
    pub position: [ExactRatio; 3],
}
#[derive(Clone, Debug)]
pub struct VerifiedDomainAtlas {
    certificate: AlgebraicDomainAtlas,
}
pub fn check_algebraic_domain_atlas(
    m: &Model,
    depth: u8,
    cuts: &[Vec<DomainPolynomial>],
    cert: &AlgebraicDomainAtlas,
) -> Result<VerifiedDomainAtlas> {
    verify_algebraic_domain_atlas(m, depth, cuts, cert)?;
    Ok(VerifiedDomainAtlas {
        certificate: cert.clone(),
    })
}
fn at(p: &[Q], x: &Q) -> Q {
    p.iter().rev().fold(Q::zero(), |v, c| v * x + c)
}
pub(crate) fn locate(p: &DomainPartition, u: &Q, v: &Q) -> Result<DomainLocation> {
    let projection = roots::arrangement_polynomial(&p.arrangement.projection)?;
    let mut cut = None;
    let mut slab = None;
    for (k, c) in p.arrangement.cuts.iter().enumerate() {
        let r = [
            roots::decode(&c.parameter[0])?,
            roots::decode(&c.parameter[1])?,
        ];
        match roots::arrangement_sign(&[-u.clone(), Q::one()], &projection, &r)? {
            0 => {
                cut = Some(k);
                break;
            }
            1 => {
                slab = k.checked_sub(1);
                break;
            }
            _ => {}
        }
    }
    let mut fiber = p
        .arrangement
        .fiber_relation
        .iter()
        .map(|r| {
            let r = r.iter().map(roots::decode).collect::<Result<Vec<_>>>()?;
            Ok(at(&r, u))
        })
        .collect::<Result<Vec<Q>>>()?;
    while fiber.last().is_some_and(Q::is_zero) {
        fiber.pop();
    }
    if fiber.is_empty() {
        return Err(fail(
            "algebraic atlas query has a nonisolated primitive fiber",
        ));
    }
    let on = at(&fiber, v).is_zero();
    let count = if v.is_zero() {
        0
    } else {
        roots::arrangement_count(&fiber, &[Q::zero(), v.clone()])?
    };
    let rank = if on && !v.is_zero() { count + 1 } else { count };
    if let Some(cut) = cut {
        if on {
            return p
                .vertices
                .iter()
                .position(|node| node.cut == cut && node.rank == rank)
                .map(|id| DomainLocation::Vertex(id as u32))
                .ok_or_else(|| fail("algebraic atlas query vertex is absent"));
        }
        return p
            .edges
            .iter()
            .position(|edge| edge.kind == DomainEdgeKind::Fiber { cut, rank })
            .map(|id| DomainLocation::Edge(id as u32))
            .ok_or_else(|| fail("algebraic atlas query critical edge is absent"));
    }
    let slab = slab.ok_or_else(|| fail("algebraic atlas query has no source slab"))?;
    if on {
        return p
            .edges
            .iter()
            .position(|edge| edge.kind == DomainEdgeKind::Section { slab, rank })
            .map(|id| DomainLocation::Edge(id as u32))
            .ok_or_else(|| fail("algebraic atlas query section is absent"));
    }
    p.faces
        .iter()
        .position(|face| face.slab == slab && face.rank == rank)
        .map(|id| DomainLocation::Face(id as u32))
        .ok_or_else(|| fail("algebraic atlas query region is absent"))
}
impl VerifiedDomainAtlas {
    pub fn certificate(&self) -> &AlgebraicDomainAtlas {
        &self.certificate
    }
    /// Exact rational queries; cells themselves retain algebraic boundaries.
    pub fn evaluate(
        &self,
        parent: usize,
        u: &ExactRatio,
        v: &ExactRatio,
    ) -> Result<DomainEvaluation> {
        let (u, v) = (roots::decode(u)?, roots::decode(v)?);
        if [&u, &v]
            .iter()
            .any(|q| q.numer().bits() > 512 || q.denom().bits() > 512)
        {
            return Err(fail("algebraic atlas query parameter bit budget exceeded"));
        }
        if u < Q::zero() || v < Q::zero() || &u + &v > Q::one() {
            return Err(fail("algebraic atlas query leaves the parent triangle"));
        }
        let p = self
            .certificate
            .partitions
            .get(parent)
            .ok_or_else(|| fail("algebraic atlas query parent is absent"))?;
        let cell = locate(p, &u, &v)?;
        let corner = if u.is_zero() && v.is_zero() {
            Some(0)
        } else if u == Q::one() {
            Some(1)
        } else if v == Q::one() {
            Some(2)
        } else {
            None
        };
        let position = if let Some(k) = corner {
            // Removable zero-weight corners use the exact source limit already
            // proved by the base atlas, never an evaluation of 0/0.
            self.certificate.base.positions[self.certificate.base.triangles[parent][k] as usize]
                .clone()
        } else {
            let mut h = self.certificate.maps[parent]
                .iter()
                .map(|r| {
                    r.iter()
                        .map(|p| {
                            Ok([
                                roots::decode(&p[0])?,
                                roots::decode(&p[1])?,
                                roots::decode(&p[2])?,
                                roots::decode(&p[3])?,
                            ])
                        })
                        .collect::<Result<Vec<_>>>()
                })
                .collect::<Result<Net>>()?;
            let a = Q::one() - &u - &v;
            while h.len() > 1 {
                let n = h.len() - 1;
                let mut next = Vec::new();
                for i in 0..n {
                    let mut row = Vec::new();
                    for j in 0..n - i {
                        let q = std::array::from_fn::<_, 4, _>(|k| {
                            &a * &h[i][j][k] + &u * &h[i + 1][j][k] + &v * &h[i][j + 1][k]
                        });
                        if q.iter()
                            .any(|q| q.numer().bits() > 8192 || q.denom().bits() > 8192)
                        {
                            return Err(fail("algebraic atlas query coordinate budget exceeded"));
                        }
                        row.push(q);
                    }
                    next.push(row);
                }
                h = next;
            }
            if h[0][0][3] <= Q::zero() {
                return Err(fail(
                    "algebraic atlas query denominator is not proved positive",
                ));
            }
            std::array::from_fn(|k| roots::ratio(&(&h[0][0][k] / &h[0][0][3])))
        };
        Ok(DomainEvaluation {
            parent: parent as u32,
            cell,
            position,
        })
    }
}
