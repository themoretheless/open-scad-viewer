//! Exact, fixed nonsingular affine equivalence followed by a fresh source audit.
//! Direct world-space contact classification remains a separate receipt.
fn point(source: &[f64], world: &[f64]) {
    assert!(source.len() == 3 && world.len() == 3);
    assert!(
        source
            .iter()
            .chain(world)
            .all(|x| x.is_finite() && (*x == 0. || (1e-100..=1e100).contains(&x.abs())))
    );
    let a = 2. * source[0];
    let b = source[1];
    let sum = a + b;
    // Knuth TwoSum; guarded normal operands exclude residual underflow.
    let bv = sum - a;
    let av = sum - bv;
    let error = (a - av) + (b - bv);
    assert_eq!(
        error, 0.,
        "Placement addition must be exact, not within tolerance"
    );
    assert_eq!(world[0], sum);
    assert_eq!(world[1], 2. * source[1]);
    assert_eq!(world[2], 2. * source[2]);
}
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let dir = std::path::PathBuf::from(std::env::args().nth(1).ok_or("skew body directory")?);
    let value: value_codec::Value =
        value_codec::from_str(&std::fs::read_to_string(dir.join("body.json"))?)?;
    let world: brep_core::Model = value_codec::from_value(value["model"].clone())?;
    let inverse = [
        [0.5, -0.25, 0., 0.],
        [0., 0.5, 0., 0.],
        [0., 0., 0.5, 0.],
        [0., 0., 0., 1.],
    ];
    let source = brep_core::transform::affine(&world, inverse)?;
    let mut replaced = world.clone();
    let mut points = 0;
    for (i, (a, b)) in source.vertices.iter().zip(&world.vertices).enumerate() {
        point(&a.point, &b.point);
        points += 1;
        replaced.vertices[i].point = a.point;
    }
    for (i, (a, b)) in source.edges.iter().zip(&world.edges).enumerate() {
        assert_eq!(a.curve.control_points.len(), b.curve.control_points.len());
        for (a, b) in a.curve.control_points.iter().zip(&b.curve.control_points) {
            point(a, b);
            points += 1;
        }
        replaced.edges[i].curve.control_points = a.curve.control_points.clone();
    }
    for (i, (a, b)) in source.faces.iter().zip(&world.faces).enumerate() {
        assert_eq!(
            a.surface.control_points.len(),
            b.surface.control_points.len()
        );
        for (a, b) in a
            .surface
            .control_points
            .iter()
            .zip(&b.surface.control_points)
        {
            assert_eq!(a.len(), b.len());
            for (a, b) in a.iter().zip(b) {
                point(a, b);
                points += 1;
            }
        }
        replaced.faces[i].surface.control_points = a.surface.control_points.clone();
    }
    // Ensures no clock, weight, trim, incidence or orientation changed.
    assert_eq!(replaced.0, source.0);
    let r = brep_core::boundary_embedding::inspect_with_linear(
        &source,
        1e-8,
        brep_core::boundary_embedding::Limits {
            exact_work: 1_000_000,
            trim_pairs: 10_000,
            trim_cells: 100_000,
            trim_domain_cells: 1_000_000,
            spans: 1024,
            contacts: brep_core::face_contacts::Limits {
                pairs: 10_000,
                cells: 100_000,
                domain_cells: 1_000_000,
                cells_per_pair: 1000,
                domain_cells_per_pair: 10_000,
            },
        },
        100_000,
    )?;
    let report = value_codec::json!({"worldBoundaryEmbeddingViaExactAffineMap":r.proven,"sourceBoundaryEmbeddingProven":r.proven,"exactControlCorrespondence":true,"unchangedClocksWeightsTrimsTopology":true,"checkedPoints":points,"determinant":8,"placement":[[2.,1.,0.,0.],[0.,2.,0.,0.],[0.,0.,2.,0.],[0.,0.,0.,1.]],"sourceTotalPairs":r.intersections.pairs.total_pairs,"sourceAllPairsClassified":r.intersections.pairs.all_pairs_classified,"sourceExactLifts":r.agreement.all_equal,"sourceExactJoins":r.agreement.all_joins_exact,"scope":"Fixed exact nonsingular affine map is a bijection on R3; it preserves all contacts and chart injectivity. World curvature and G2 need separate fresh checks; the direct world pair inspector is not upgraded."});
    std::fs::write(
        dir.join("affine-embedding.json"),
        value_codec::to_string_pretty(&report)?,
    )?;
    assert!(r.proven, "Source audit unresolved; no affine admission");
    println!("worldBoundaryEmbeddingViaExactAffineMap=true");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::point;
    #[test]
    fn exact_dyadic_placement_is_accepted() {
        point(&[0.125, 0.375, 0.5], &[0.625, 0.75, 1.]);
    }
    #[test]
    #[should_panic]
    fn one_ulp_world_perturbation_refuses() {
        point(
            &[0.125, 0.375, 0.5],
            &[f64::from_bits(0.625f64.to_bits() + 1), 0.75, 1.],
        );
    }
    #[test]
    #[should_panic]
    fn rounded_forward_addition_cannot_claim_exact_map() {
        point(&[0.5, 2f64.powi(-54), 1.], &[1., 2f64.powi(-53), 2.]);
    }
}
