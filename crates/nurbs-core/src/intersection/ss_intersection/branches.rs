//! Native classification and coverage rules for surface-intersection branches.
#[derive(Clone, Copy)]
pub(super) enum BranchKind {
    Empty,
    Curve,
    Overlap,
}
#[derive(Clone, Copy)]
pub(super) enum Contact {
    Admitted,
    Singular,
    Unknown,
}
pub(super) struct BranchDescriptor {
    pub kind: BranchKind,
    pub contact: Contact,
    pub closed: bool,
}
#[derive(Clone, Debug)]
pub struct Summary {
    pub component_count: usize,
    pub closed_loops: usize,
    pub overlap_regions: usize,
    pub singular_strata: usize,
    pub permits_topology_authorship: bool,
}

pub(super) fn summarize(branches: &[BranchDescriptor], complete: bool) -> Summary {
    let mut summary = Summary {
        component_count: 0,
        closed_loops: 0,
        overlap_regions: 0,
        singular_strata: 0,
        permits_topology_authorship: complete,
    };
    for branch in branches {
        if !matches!(branch.kind, BranchKind::Empty) {
            summary.component_count += 1;
            summary.permits_topology_authorship &= matches!(branch.contact, Contact::Admitted);
        }
        if branch.closed && matches!(branch.kind, BranchKind::Curve | BranchKind::Overlap) {
            summary.closed_loops += 2;
        }
        summary.overlap_regions += usize::from(matches!(branch.kind, BranchKind::Overlap));
        summary.singular_strata += usize::from(matches!(branch.contact, Contact::Singular));
    }
    summary
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn completeness_and_contact_class_both_gate_authority() {
        let branches = [BranchDescriptor {
            kind: BranchKind::Curve,
            contact: Contact::Admitted,
            closed: true,
        }];
        assert!(summarize(&branches, true).permits_topology_authorship);
        assert!(!summarize(&branches, false).permits_topology_authorship);
        assert_eq!(summarize(&branches, true).closed_loops, 2);
        for contact in [Contact::Singular, Contact::Unknown] {
            assert!(
                !summarize(
                    &[BranchDescriptor {
                        kind: BranchKind::Overlap,
                        contact,
                        closed: false
                    }],
                    true
                )
                .permits_topology_authorship
            );
        }
    }
    #[test]
    fn empty_components_are_excluded_from_graph_but_preserve_strata_counts() {
        let report = summarize(
            &[BranchDescriptor {
                kind: BranchKind::Empty,
                contact: Contact::Singular,
                closed: true,
            }],
            true,
        );
        assert_eq!(report.component_count, 0);
        assert_eq!(report.closed_loops, 0);
        assert_eq!(report.singular_strata, 1);
        assert!(report.permits_topology_authorship);
    }
}
