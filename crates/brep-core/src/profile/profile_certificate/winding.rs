//! Exact nonzero-fill regularization of separated simple original NURBS wires.
use super::*;
mod periodic;
pub use periodic::{
    ApproximatePeriodicWindingRegions, CoherentPeriodicWindingRegions, PeriodicWindingRegions,
    SurfacePeriodicRegions, bind_periodic_regions_to_surface,
    regularize_approximated_periodic_winding, regularize_coherent_periodic_winding,
    regularize_periodic_winding, regularize_projective_coherent_periodic_winding,
};
pub use periodic::{
    ClockMappedPeriodicRegions, ClockMappedSurfaceRegions, ExactPeriodicContactRegions,
    LinearPeriodicContactRegions, MultispanChartAlias, MultispanPeriodicRegions,
    MultispanSurfaceRegions, OwnedClockMappedFaces, OwnedMultispanPeriodicFaces,
    OwnedPeriodicFaces, PeriodicFaceLimits, assemble_clock_mapped_surface_faces,
    assemble_multispan_periodic_surface_faces, assemble_periodic_surface_faces,
    bind_clock_mapped_regions_to_surface, bind_multispan_regions_to_surface,
    regularize_clock_mapped_periodic_winding, regularize_exact_periodic_contacts,
    regularize_linear_periodic_contacts, regularize_multispan_periodic_winding,
};

#[derive(Clone, Debug)]
pub struct WindingComponent {
    wires: Vec<Vec<Curve>>,
    source_loops: Vec<usize>,
    reversed: Vec<bool>,
    certificate: ProfileCertificate,
    verified: VerifiedProfile,
}
impl WindingComponent {
    pub fn wires(&self) -> &[Vec<Curve>] {
        &self.wires
    }
    pub fn source_loops(&self) -> &[usize] {
        &self.source_loops
    }
    pub fn reversed(&self) -> &[bool] {
        &self.reversed
    }
    pub fn verified_profile(&self) -> &VerifiedProfile {
        &self.verified
    }
}
#[derive(Clone, Debug)]
pub struct WindingRegularization {
    source: Vec<Vec<Curve>>,
    coefficients: Vec<i32>,
    tolerance: f64,
    geometry: ProfileCertificate,
    /// [winding just outside this wire, winding just inside this wire].
    winding: Vec<[i64; 2]>,
    components: Vec<WindingComponent>,
    removed: Vec<usize>,
}
impl WindingRegularization {
    pub fn source(&self) -> &[Vec<Curve>] {
        &self.source
    }
    pub fn coefficients(&self) -> &[i32] {
        &self.coefficients
    }
    pub fn winding(&self) -> &[[i64; 2]] {
        &self.winding
    }
    pub fn components(&self) -> &[WindingComponent] {
        &self.components
    }
    pub fn removed_source_loops(&self) -> &[usize] {
        &self.removed
    }
    /// Search-free replay of the complete original geometry witness, exact
    /// nesting/winding selection, source bindings and every material-left face.
    pub fn verify(&self) -> Result<()> {
        let claims = classify(
            &self.source,
            &self.coefficients,
            self.tolerance,
            &self.geometry,
        )?;
        if self.winding != claims.winding
            || self.removed != claims.removed
            || self.components.len() != claims.groups.len()
        {
            return Err(refused(
                "Winding regularization omits or changes a region claim",
            ));
        }
        for (component, ids) in self.components.iter().zip(&claims.groups) {
            let (wires, reversed) = material_wires(&self.source, &claims.orientation, ids);
            if component.source_loops != *ids
                || component.wires != wires
                || component.reversed != reversed
            {
                return Err(refused(
                    "Regularized component is not bound to its original wires",
                ));
            }
            let verified = super::verify(&component.wires, self.tolerance, &component.certificate)?;
            if verified != component.verified {
                return Err(refused("Regularized component profile receipt changed"));
            }
        }
        Ok(())
    }
}
struct Claims {
    orientation: Vec<i8>,
    winding: Vec<[i64; 2]>,
    groups: Vec<Vec<usize>>,
    removed: Vec<usize>,
}
fn classify(
    loops: &[Vec<Curve>],
    coefficients: &[i32],
    tolerance: f64,
    certificate: &ProfileCertificate,
) -> Result<Claims> {
    if loops.len() != coefficients.len()
        || coefficients.iter().any(|&c| i64::from(c).abs() > 1_000_000)
    {
        return Err(refused(
            "Winding coefficients require one bounded integer per source wire",
        ));
    }
    let (verified, orientation) = verify_geometry(loops, tolerance, certificate)?;
    let p = pieces(loops, certificate.subdivision_depth)?;
    let rings: Vec<Vec<_>> = (0..loops.len())
        .map(|w| {
            p.iter()
                .filter(|p| p.wire == w)
                .map(|p| &p.points[0])
                .collect()
        })
        .collect();
    let mut contains = vec![vec![false; loops.len()]; loops.len()];
    let mut winding = Vec::new();
    for i in 0..loops.len() {
        let mut outside = 0_i64;
        let mut depth = 0;
        for j in 0..loops.len() {
            if i != j {
                contains[j][i] = polygon_winding(&rings[j], rings[i][0])? != 0;
                if contains[j][i] {
                    outside += i64::from(orientation[j]) * i64::from(coefficients[j]);
                    depth += 1;
                }
            }
        }
        if depth != verified.nesting_depth()[i] {
            return Err(refused(
                "Exact winding containment differs from source geometry proof",
            ));
        }
        winding.push([
            outside,
            outside + i64::from(orientation[i]) * i64::from(coefficients[i]),
        ]);
    }
    let outers: Vec<_> = (0..loops.len())
        .filter(|&i| winding[i][0] == 0 && winding[i][1] != 0)
        .collect();
    let mut groups: Vec<_> = outers.iter().map(|&i| vec![i]).collect();
    let mut removed = Vec::new();
    for i in 0..loops.len() {
        if winding[i][0] != 0 && winding[i][1] == 0 {
            let owner = outers
                .iter()
                .enumerate()
                .filter(|&(_, &j)| contains[j][i])
                .max_by_key(|&(_, &j)| verified.nesting_depth()[j])
                .map(|(group, _)| group)
                .ok_or_else(|| refused("Regularized hole has no containing material owner"))?;
            groups[owner].push(i);
        } else if winding[i][0] != 0 || winding[i][1] == 0 {
            removed.push(i);
        }
    }
    Ok(Claims {
        orientation,
        winding,
        groups,
        removed,
    })
}
fn reverse_exact(c: &Curve) -> Curve {
    let mut out = c.clone();
    out.control_points.reverse();
    out.weights.reverse();
    out.knots = c.knots.iter().rev().map(|x| -*x).collect();
    out
}
fn material_wires(
    source: &[Vec<Curve>],
    orientation: &[i8],
    ids: &[usize],
) -> (Vec<Vec<Curve>>, Vec<bool>) {
    let reversed: Vec<_> = ids
        .iter()
        .enumerate()
        .map(|(j, &i)| orientation[i] != if j == 0 { 1 } else { -1 })
        .collect();
    let wires = ids
        .iter()
        .zip(&reversed)
        .map(|(&i, &reverse)| {
            if reverse {
                source[i].iter().rev().map(reverse_exact).collect()
            } else {
                source[i].clone()
            }
        })
        .collect();
    (wires, reversed)
}
/// Normalize the nonzero-fill region of 0..32 separated simple original wires
/// with bounded integer coefficients. Complete source span/pair proofs are
/// required, including zero-coefficient wires. Internal nonzero/nonzero borders
/// are removed; output components have one CCW outer and CW holes, retaining
/// source curves or exact parameter-negated reversals. Winding multiplicities
/// are deliberately normalized to filled regions. Crossings, coincident or
/// touching distinct wires remain unresolved, as do supporting-surface and
/// 3D face admission. This does not weaken ordinary material-left verification.
pub fn regularize_nonzero_winding(
    loops: &[Vec<Curve>],
    coefficients: &[i32],
    tolerance: f64,
) -> Result<WindingRegularization> {
    if loops.len() != coefficients.len()
        || coefficients.iter().any(|&c| i64::from(c).abs() > 1_000_000)
    {
        return Err(refused("Winding coefficient count or magnitude is invalid"));
    }
    let geometry = producer::certify_geometry(loops, tolerance)?;
    let claims = classify(loops, coefficients, tolerance, &geometry)?;
    let mut components = Vec::new();
    for ids in &claims.groups {
        let (wires, reversed) = material_wires(loops, &claims.orientation, ids);
        let certificate = certify(&wires, tolerance)?;
        let verified = super::verify(&wires, tolerance, &certificate)?;
        components.push(WindingComponent {
            wires,
            source_loops: ids.clone(),
            reversed,
            certificate,
            verified,
        });
    }
    let result = WindingRegularization {
        source: loops.to_vec(),
        coefficients: coefficients.to_vec(),
        tolerance,
        geometry,
        winding: claims.winding,
        components,
        removed: claims.removed,
    };
    result.verify()?;
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn replay_rejects_stale_winding_removed_source_owner_and_output_claims() {
        let a =
            crate::sketch::polygon_wire(vec![[0., 0.], [10., 0.], [10., 10.], [0., 10.]]).unwrap();
        let b = crate::sketch::polygon_wire(vec![[2., 2.], [8., 2.], [8., 8.], [2., 8.]]).unwrap();
        let result = regularize_nonzero_winding(&[a, b], &[1, 1], 1e-7).unwrap();
        let mut stale = result.clone();
        stale.winding[0][1] = 2;
        assert!(stale.verify().is_err());
        let mut stale = result.clone();
        stale.removed.clear();
        assert!(stale.verify().is_err());
        let mut stale = result.clone();
        stale.coefficients[1] = -1;
        assert!(stale.verify().is_err());
        let mut stale = result.clone();
        stale.components[0].source_loops[0] = 1;
        assert!(stale.verify().is_err());
        let mut stale = result.clone();
        stale.components[0].wires[0][0].control_points[0][0] = 1.;
        assert!(stale.verify().is_err());
        let mut stale = result.clone();
        stale.components[0].certificate.nesting_depth[0] = 2;
        assert!(stale.verify().is_err());
        let mut stale = result.clone();
        stale.source[0][0].weights[0] = 2.;
        assert!(stale.verify().is_err());
        let mut stale = result.clone();
        stale.tolerance = 2e-7;
        assert!(stale.verify().is_err());
        result.verify().unwrap();
    }
}
