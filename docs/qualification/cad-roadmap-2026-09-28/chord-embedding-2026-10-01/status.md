# Represented chord graph admission

Before directed boundary traversal, the represented graph must pass pairwise embedding checks. Graph coordinates are finite and bounded; coincident vertex coordinates require reconciliation. Edge endpoints must refer to distinct valid vertices. Every edge pair must be proved separated or meet exclusively at one shared topological vertex. Unjoined crossings, contacts, overlaps and unresolved predicates refuse admission. Pair budgets are explicit and failure returns no accepted partial graph.

Eleven chord pipeline tests passed: intersection construction, graph splitting, directed walks, and embedding admission. Embedding fixtures cover an unjoined crossing, a T contact, duplicate coordinates, allowed shared endpoints/separated edges, and resource refusal. Existing bowtie walk/split tests also pass through mandatory embedding admission.

This proves only admission of the represented binary64 straight-edge graph. It does not prove the original offset topology across construction uncertainty. Winding selection, containment/holes, contact reconciliation and original-offset deviation/topology gates remain open. No trimmed offset command has been published.
