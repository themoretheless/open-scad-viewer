//! Closed exact graph columns and their bounded algebraic lens intersection.
use super::*;
#[derive(Clone, Debug)]
pub struct GraphSlab {
    carriers: [Surface; 2],
}
fn difference(a: &[Bi; 4], b: &[Bi; 4], w: &mut Work) -> Result<Bi> {
    sum(
        &product(&a[2], &b[3], w)?,
        &scale(&product(&b[2], &a[3], w)?, &-Q::one()),
    )
}
impl GraphSlab {
    /// Material over the upper carrier's closed normalized triangle, between
    /// lower and upper graphs. The vertical column map has a strictly positive
    /// Jacobian; ruled side boundaries are part of this exact source solid.
    pub fn new(lower: Surface, upper: Surface) -> Result<Self> {
        let mut work = Work::default();
        let aligned = align(&upper, &lower, &mut work)?;
        if !polynomial::positive(
            &difference(&aligned.powers[0], &aligned.powers[1], &mut work)?,
            &mut work,
        )? {
            return Err(fail("Graph slab thickness is not proved strictly positive"));
        }
        Ok(Self {
            carriers: [lower, upper],
        })
    }
    pub fn lower(&self) -> &Surface {
        &self.carriers[0]
    }
    pub fn upper(&self) -> &Surface {
        &self.carriers[1]
    }
    pub fn reverify(&self) -> Result<()> {
        Self::new(self.lower().clone(), self.upper().clone()).map(|_| ())
    }
}
/// Exact intersection when the upper/lower envelopes are proved to be the
/// first slab's upper and second slab's lower, with every positive component
/// strictly interior to the first footprint. Other envelope orders refuse.
pub fn intersect_graph_slabs(
    a: &GraphSlab,
    b: &GraphSlab,
    multiplier: &DomainPolynomial,
) -> Result<AlgebraicRegionBrep> {
    a.reverify()?;
    b.reverify()?;
    let mut work = Work::default();
    let al = align(a.upper(), a.lower(), &mut work)?;
    let bl = align(a.upper(), b.lower(), &mut work)?;
    let bu = align(a.upper(), b.upper(), &mut work)?;
    // The first triangular footprint must be inside the SECOND triangular
    // footprint, not merely inside its supporting NURBS parameter square.
    for corner in 0..3 {
        let uv = bu.maps[1].each_ref().map(|r| {
            if corner == 0 {
                r[0].clone()
            } else {
                &r[0] + &r[corner]
            }
        });
        if uv[0] < Q::zero() || uv[1] < Q::zero() || &uv[0] + &uv[1] > Q::one() {
            return Err(fail(
                "Graph slab footprints require an additional exact planar overlay",
            ));
        }
    }
    if !polynomial::positive(
        &difference(&bl.powers[1], &al.powers[1], &mut work)?,
        &mut work,
    )? || !polynomial::positive(
        &difference(&bu.powers[1], &al.powers[0], &mut work)?,
        &mut work,
    )? {
        return Err(fail("Graph slab intersection envelope order is unproved"));
    }
    // The old region verifier checks ALL positive cells, exact source seams,
    // regularity and closed cap pairs. Empty/boundary-reaching results refuse.
    let equation = encode(&difference(&bl.powers[0], &bl.powers[1], &mut work)?);
    // Keep the triangle boundary factors separate during projection. The one
    // source factor is the COMPLETE exact height numerator, not a fitted or
    // presumed factorization. The existing checker verifies the full product.
    let region =
        certify_graph_region_brep_factored(a.upper(), b.lower(), multiplier, &[equation], None)?;
    if region.certificate().faces.is_empty() {
        return Err(fail(
            "No bounded positive algebraic lens in this graph slab order",
        ));
    }
    Ok(region)
}
impl GraphSlab {
    pub fn source_ids(&self) -> Result<[String; 2]> {
        Ok([source(self.lower())?, source(self.upper())?])
    }
}
