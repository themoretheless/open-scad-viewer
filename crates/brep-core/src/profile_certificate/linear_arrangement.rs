//! Original NURBS line-image chains through exact weighted contact arrangement.
//! Curved spans refuse: no chord replaces an unproved original curved carrier.
use super::*;
use algebraic_intersections::{ExactRatio, ratio};
use offset_regions::arrangement::{self, Rings};
#[derive(Clone, Debug)]
pub struct LinearWindingArrangement {
    source: Vec<Vec<Curve>>,
    coefficients: Vec<i32>,
    source_rings: Rings,
    regions: Rings,
    groups: Vec<Vec<usize>>,
}
fn inputs(source: &[Vec<Curve>], coefficients: &[i32]) -> Result<Rings> {
    source_bits(source, 1e-7)?;
    if source.len() != coefficients.len()
        || coefficients.iter().any(|&c| i64::from(c).abs() > 1_000_000)
    {
        return Err(refused(
            "Linear arrangement needs one bounded coefficient per original wire",
        ));
    }
    let nets = extraction::nets(source)?;
    let endpoint = |n: &Vec<[Q; 3]>, end: bool| {
        let p = if end { n.last().unwrap() } else { &n[0] };
        [&p[0] / &p[2], &p[1] / &p[2]]
    };
    let mut rings = vec![Vec::new(); source.len()];
    for wire in 0..source.len() {
        let cells = nets
            .iter()
            .filter(|(w, _)| *w == wire)
            .map(|(_, n)| n)
            .collect::<Vec<_>>();
        if cells.is_empty()
            || (0..cells.len())
                .any(|i| endpoint(cells[i], true) != endpoint(cells[(i + 1) % cells.len()], false))
        {
            return Err(refused("Original linear chain is not exactly closed"));
        }
        for net in cells {
            let a = endpoint(net, false);
            let b = endpoint(net, true);
            let delta = [&b[0] - &a[0], &b[1] - &a[1]];
            if a == b {
                if net.iter().any(|h| [&h[0] / &h[2], &h[1] / &h[2]] != a) {
                    return Err(refused(
                        "Closed nonlinear carrier is not a constant line image",
                    ));
                }
                continue;
            }
            if net.iter().any(|h| {
                (&h[0] / &h[2] - &a[0]) * &delta[1] - (&h[1] / &h[2] - &a[1]) * &delta[0]
                    != Q::zero()
            }) {
                return Err(refused(
                    "Original curved carrier cannot be replaced by a chord",
                ));
            }
            let axis = usize::from(delta[0].is_zero());
            if !arithmetic::monotone(net, axis)? {
                return Err(refused(
                    "Original line image has no verified complete oriented traversal",
                ));
            }
            rings[wire].push(a);
        }
    }
    Ok(rings)
}
fn encode(rings: &Rings) -> Vec<Vec<[ExactRatio; 2]>> {
    rings
        .iter()
        .map(|r| r.iter().map(|p| [ratio(&p[0]), ratio(&p[1])]).collect())
        .collect()
}
impl LinearWindingArrangement {
    pub fn source(&self) -> &[Vec<Curve>] {
        &self.source
    }
    pub fn coefficients(&self) -> &[i32] {
        &self.coefficients
    }
    /// Exact rational vertices; no binary64 rounding or topology claim on a
    /// rounded display polygon is inherited from this result.
    pub fn exact_regions(&self) -> ExactRegionRings {
        encode(&self.regions)
    }
    pub fn groups(&self) -> &[Vec<usize>] {
        &self.groups
    }
    pub fn reverify_source(&self) -> Result<()> {
        let source = inputs(&self.source, &self.coefficients)?;
        let regions = arrangement::nonzero(&source, &self.coefficients)?;
        let groups = arrangement::groups(&regions)?;
        if source != self.source_rings || regions != self.regions || groups != self.groups {
            return Err(refused(
                "Original line-image contact arrangement receipt changed",
            ));
        }
        Ok(())
    }
}
/// Regularize actual original line-image NURBS wires, preserving arbitrary
/// bounded signed winding coefficients. Complete rational intersections,
/// overlaps, endpoint contacts, halfedge fans and nonzero cell selection are
/// replayed. Higher-degree rational nonlinear PARAMETERIZATION is supported
/// only after exact collinearity and strict monotone image traversal proof.
/// General curved carriers and 3D supporting faces remain separate.
pub fn regularize_linear_winding_arrangement(
    source: &[Vec<Curve>],
    coefficients: &[i32],
) -> Result<LinearWindingArrangement> {
    let source_rings = inputs(source, coefficients)?;
    let regions = arrangement::nonzero(&source_rings, coefficients)?;
    let groups = arrangement::groups(&regions)?;
    let owner = LinearWindingArrangement {
        source: source.to_vec(),
        coefficients: coefficients.to_vec(),
        source_rings,
        regions,
        groups,
    };
    owner.reverify_source()?;
    Ok(owner)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn original_linear_arrangement_replay_rejects_missing_or_changed_regions() {
        let curve = Curve::from_polyline(vec![
            vec![0., 0.],
            vec![2., 0.],
            vec![2., 2.],
            vec![0., 2.],
            vec![0., 0.],
        ])
        .unwrap();
        let owner = regularize_linear_winding_arrangement(&[vec![curve]], &[1]).unwrap();
        let mut bad = owner.clone();
        bad.regions.clear();
        assert!(bad.reverify_source().is_err());
        let mut bad = owner.clone();
        bad.groups.clear();
        assert!(bad.reverify_source().is_err());
        let mut bad = owner.clone();
        bad.source_rings[0][0][0] += Q::one();
        assert!(bad.reverify_source().is_err());
        let mut bad = owner.clone();
        bad.coefficients[0] = 0;
        assert!(bad.reverify_source().is_err());
        let mut bad = owner;
        bad.source[0][0].control_points[1][0] = 2_f64.next_up();
        assert!(bad.reverify_source().is_err());
    }
}
