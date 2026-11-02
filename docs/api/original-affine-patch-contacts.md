# Original affine-patch contacts

Rust API: `nurbs_core::intersection::certified_curve_surface::original_affine_patch_contacts(curve, patch, parameter_tolerance, max_spans, max_exact_work)`.

This read-only operation isolates curve contacts with an exact untrimmed affine parallelogram, including scalar multiplicities greater than one. The plane equation is constructed inside the exact predicate from the four original vertices; its normal and offset need not have a binary64 representation.

Inputs are valid nonperiodic 3D positive-weight sources. The curve admits degree 1..25, at most 256 controls, and C0 or smoother interior knot storage. The patch must be a single clamped degree-(1,1), uniform-weight 2x2 chart with an exact parallelogram identity. An approximate plane is not substituted for an unsupported chart.

`parameter_tolerance` is positive and finite in the natural curve parameter. `max_spans` (1..100000) bounds scalar span queries; `max_exact_work` is one shared predicate allowance across all queried spans. Unvisited, indeterminate and coincident spans remain in `unresolved_spans`. Scalar decisions and their source context identities remain in `scalar_proofs`.

Each contact contains a natural `[curve-t, patch-u, patch-v]` parameter box and world position bounds. `source_spans` and `multiplicities` preserve the one-sided orders. Only identical exact knot parameters alias into one event; overlapping nondegenerate enclosures are not merged. No transverse crossing sign is inferred from an even-multiplicity contact.

Finite patch membership is proved by an outward interval affine inverse. Exterior roots are excluded; uncertain boundary membership remains in `ambiguous_parameters`. `complete` requires both unresolved lists to be empty, and covers only the given curve against the untrimmed patch. Coincident curve intervals are not converted to isolated contacts. No trim, Body, whole-surface branch or general freeform tangency admission follows from this report.

## Bridge

Operation `nurbs_original_affine_patch_contacts` requires exactly `op`, `curve`, `patch`, `tolerance`, `maxSpans`, and `maxExactWork`. The exact-work allowance is 0..`cad-predicates::MAX_WORK`; zero returns unresolved coverage rather than inventing absence.

Schema `nurbs-original-affine-patch-contacts/1` returns `contacts` with parameter/world boxes and parallel `sourceSpans`/`multiplicities` arrays. `scalarProofs` summarizes original-span polynomial outcomes, normalized original-span root intervals, orders and charged work. The opaque in-process cache context is deliberately not serialized (`proofContextSerialized:false`); these summaries are not portable proof certificates.

Status is `present` when any finite-patch contact is admitted, `absent-on-patch` only when complete coverage contains no contacts, and `unresolved` otherwise. Presence can coexist with incomplete coverage. `unresolvedSpans`, `ambiguousParameters`, `exactWork`, and `spans` retain missing coverage and resource evidence. Whole-surface branch coverage, trim and Body admission remain false. No transverse `crossingSign` is emitted for a scalar contact.

Browser adapter: `SurfaceSelfIntersectionBrowser.inspectOriginalPatchContacts(query, options)` uses the same owned Worker lane as surface and iso–target diagnostics. It validates `OriginalPatchContactsReport`, including one-sided order/source-span links and exact-work accounting, and checks the requested span/exact-work budgets on reply. A zero exact budget is a valid unresolved query. Abort, deadline and busy handling are shared. The caller supplies the import-free module; this adapter does not install a package or register a production provider.
