use super::OriginalQuadricCarrierRoot;
pub(super) fn merge(roots: &mut Vec<OriginalQuadricCarrierRoot>, root: OriginalQuadricCarrierRoot) {
    if root.parameter[0] == root.parameter[1] {
        if let Some(old) = roots.iter_mut().find(|p| p.parameter == root.parameter) {
            for (a, b) in old.position_bounds.iter_mut().zip(root.position_bounds) {
                a[0] = a[0].min(b[0]);
                a[1] = a[1].max(b[1]);
            }
            old.source_spans.extend(root.source_spans);
            old.multiplicities.extend(root.multiplicities);
            old.normalized_original_span_parameters
                .extend(root.normalized_original_span_parameters);
            return;
        }
    }
    roots.push(root);
}
