//! STEP and IGES import entry points that apply the kernel import policy after parsing.
use brep_core::import_policy::{ImportBudget, ImportOutcome, ImportPolicy, ImportResult, apply_import_policy};

/// Additive STEP facade; legacy `import_step_v*` signatures are unchanged.
pub fn import_step_with_policy(
    text: &str,
    policy: ImportPolicy,
    budget: ImportBudget,
) -> ImportResult<ImportOutcome> {
    let (model, _, _) = crate::step_interchange_v3::import_step_v3(text)?;
    apply_import_policy(model, policy, budget)
}

/// Additive IGES facade using the same post-parse routing as STEP.
pub fn import_iges_v2_with_policy(
    text: &str,
    policy: ImportPolicy,
    budget: ImportBudget,
) -> ImportResult<ImportOutcome> {
    let (model, _, _) = crate::iges_interchange_v2::import_iges_v2(text)?;
    apply_import_policy(model, policy, budget)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn clean_step_file_passes_every_policy() {
        let model = brep_core::cuboid([0.; 3], [2.; 3]).unwrap();
        let (text, _, _) = crate::export_step_v3(&model).unwrap();
        assert_eq!(ImportPolicy::default(), ImportPolicy::RepairWithReport);
        for policy in [
            ImportPolicy::Strict,
            ImportPolicy::RepairWithReport,
            ImportPolicy::BestEffort,
        ] {
            let outcome = import_step_with_policy(&text, policy, ImportBudget::default()).unwrap();
            assert_eq!(outcome.policy, policy);
            assert!(outcome.defects.complete);
            assert!(outcome.defects.defects.is_empty());
            assert!(outcome.repair.is_none());
            outcome.model.validate().unwrap();
        }
    }
}
