//! Source-preserving rational G0 wire construction. Only explicitly editable
//! pieces move; optional exact tangent and line-contact repairs are explicit. Neither material inclusion nor a sewn Body is implied.
use super::*;
mod contacts;
pub mod jordan;
pub mod material;
pub mod nesting;
pub mod prism;
pub mod radius;
pub mod regularity;
pub mod smooth;
mod tangent_repair;
pub use contacts::LineContact;
pub use smooth::SmoothEdits;
#[derive(Clone, Debug)]
pub struct Wire {
    sources: Vec<Restriction>,
    editable: Vec<bool>,
    tolerance: Q,
    pieces: Vec<Vec<Span>>,
    displacement_squared: Q,
    tangent_repair: bool,
    contacts: Vec<LineContact>,
}
fn endpoint(spans: &[Span], end: bool) -> [Q; 2] {
    let h = if end {
        spans.last().unwrap().net.last().unwrap()
    } else {
        &spans[0].net[0]
    };
    [&h[0] / &h[2], &h[1] / &h[2]]
}
fn set_endpoint(spans: &mut [Span], end: bool, point: &[Q; 2]) {
    let h = if end {
        spans.last_mut().unwrap().net.last_mut().unwrap()
    } else {
        &mut spans[0].net[0]
    };
    h[0] = &point[0] * &h[2];
    h[1] = &point[1] * &h[2];
}
fn derive(
    sources: &[Restriction],
    editable: &[bool],
    tolerance: &Q,
    repair_tangents: bool,
    contacts: &[LineContact],
) -> Result<(Vec<Vec<Span>>, Q)> {
    if sources.is_empty()
        || sources.len() > 256
        || editable.len() != sources.len()
        || tolerance < &Q::zero()
        || sources.iter().map(|s| s.spans.len()).sum::<usize>() > 4096
    {
        return Err(refused("Invalid exact wire source, edit mask or budget"));
    }
    for s in sources {
        s.reverify()?;
    }
    let mut pieces = sources.iter().map(|s| s.spans.clone()).collect::<Vec<_>>();
    for i in 0..pieces.len() {
        let j = (i + 1) % pieces.len();
        let a = endpoint(&pieces[i], true);
        let b = endpoint(&pieces[j], false);
        if a == b {
            continue;
        }
        match (editable[i], editable[j]) {
            (false, false) => {
                return Err(refused("Fixed original wire endpoints do not join exactly"));
            }
            (true, false) => set_endpoint(&mut pieces[i], true, &b),
            // At a flexible/flexible join the preceding endpoint is canonical.
            (_, true) => set_endpoint(&mut pieces[j], false, &a),
        }
    }
    if repair_tangents {
        contacts::apply(contacts, editable, &mut pieces)?;
        tangent_repair::apply(sources, editable, &mut pieces, contacts)?;
    }
    let mut maximum = Q::zero();
    for (i, piece) in pieces.iter().enumerate() {
        if !editable[i] && *piece != sources[i].spans {
            return Err(refused("Fixed source wire piece changed"));
        }
        for (span, original) in piece.iter().zip(&sources[i].spans) {
            for (a, b) in span.net.iter().zip(&original.net) {
                if a[2] != b[2] {
                    return Err(refused("Exact wire repair changed a source weight"));
                }
                let distance = (0..2)
                    .map(|axis| {
                        let delta = &a[axis] / &a[2] - &b[axis] / &b[2];
                        &delta * &delta
                    })
                    .sum::<Q>();
                maximum = maximum.max(distance);
            }
        }
        if endpoint(piece, true) != endpoint(&pieces[(i + 1) % pieces.len()], false) {
            return Err(refused("Exact rational wire is not closed"));
        }
    }
    // Each edited rational Bezier span has unchanged positive weights. Its
    // displacement is a convex combination of Cartesian control displacements,
    // so this maximum over ALL controls bounds the WHOLE edited span, including interiors.
    if maximum > tolerance * tolerance {
        return Err(refused("Whole-curve wire adjustment exceeds tolerance"));
    }
    Ok((pieces, maximum))
}
pub fn sew(sources: Vec<Restriction>, editable: Vec<bool>, tolerance: ExactRatio) -> Result<Wire> {
    let tolerance = decode_ratio(&tolerance)?;
    let (pieces, displacement_squared) = derive(&sources, &editable, &tolerance, false, &[])?;
    Ok(Wire {
        sources,
        editable,
        tolerance,
        pieces,
        displacement_squared,
        tangent_repair: false,
        contacts: Vec::new(),
    })
}
/// Explicit source-bound repair of quadratic tangent controls, with a whole
/// Cartesian displacement bound over all original controls and unchanged weights.
pub fn sew_tangent_repaired(
    sources: Vec<Restriction>,
    editable: Vec<bool>,
    tolerance: ExactRatio,
) -> Result<Wire> {
    sew_tangent_with_contacts(sources, editable, tolerance, Vec::new())
}
pub fn sew_tangent_with_contacts(
    sources: Vec<Restriction>,
    editable: Vec<bool>,
    tolerance: ExactRatio,
    contacts: Vec<LineContact>,
) -> Result<Wire> {
    let tolerance = decode_ratio(&tolerance)?;
    let (pieces, displacement_squared) = derive(&sources, &editable, &tolerance, true, &contacts)?;
    let wire = Wire {
        sources,
        editable,
        tolerance,
        pieces,
        displacement_squared,
        tangent_repair: true,
        contacts,
    };
    wire.qualify_smooth_edits(["0".into(), "1".into()])?;
    Ok(wire)
}
impl Wire {
    pub fn line_contacts(&self) -> &[LineContact] {
        &self.contacts
    }
    pub fn tangent_repaired(&self) -> bool {
        self.tangent_repair
    }
    pub fn sources(&self) -> &[Restriction] {
        &self.sources
    }
    pub fn editable(&self) -> &[bool] {
        &self.editable
    }
    pub fn pieces(&self) -> &[Vec<Span>] {
        &self.pieces
    }
    pub fn displacement_squared_bound(&self) -> ExactRatio {
        ratio(&self.displacement_squared)
    }
    pub fn tolerance(&self) -> ExactRatio {
        ratio(&self.tolerance)
    }
    pub fn reverify(&self) -> Result<()> {
        let (pieces, maximum) = derive(
            &self.sources,
            &self.editable,
            &self.tolerance,
            self.tangent_repair,
            &self.contacts,
        )?;
        if pieces != self.pieces || maximum != self.displacement_squared {
            return Err(refused("Original rational wire replay changed"));
        }
        Ok(())
    }
}
