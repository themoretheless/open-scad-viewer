use super::*;

/// Successor import with explicit SI/context, placement and identity reporting.
pub fn import_step_v2(text: &str) -> Result<(Model, FeatureCertificate, StepIdentityReport)> {
    validate_v2_finite_subset(text)?;
    let scale = v2_length_scale(text)?;
    let placement = v2_nested_placement(text)?;
    let (mut model, _) = import_step(text)?;
    if let Some(matrix) = placement {
        model = transform::affine(&model, matrix)?;
    }
    if (scale - 1.).abs() > f64::EPSILON {
        model = transform::affine(
            &model,
            [
                [scale, 0., 0., 0.],
                [0., scale, 0., 0.],
                [0., 0., scale, 0.],
                [0., 0., 0., 1.],
            ],
        )?;
    }
    let identity = restore_v2_identity(text, &mut model)?;
    model.validate()?;
    Ok((
        model,
        FeatureCertificate {
            capability: STEP_INTERCHANGE_V2_CAPABILITY,
            complete: true,
            notes: vec![
                "si_unit_context",
                "nested_rigid_placement",
                "topology_provenance_mapping",
                "explicit_identity_report",
            ],
        },
        identity,
    ))
}
