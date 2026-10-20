# Original physical faces candidate

Catalog remains 198/200. All Rust changes are isolated in /private/tmp/nurbs-native-200-next-6v12adcx. The cumulative patch applies against the verified baseline, not after previous cumulative patches.

OwnedOriginalPhysicalFaces owns the original surface and original pcurves with exact algebraic cuts. World edge geometry is S(original_pcurve(t)); no fitted spatial control nets or rounded endpoints. Compact physical vertices, seam aliases, shared edge traces, coedge uses, patches, holes, connected physical faces, boundary cycles and Euler characteristics are derived and replayed from the complete source proof chain. Different seam clocks retain their own traces.

Current checks: 102 profile tests and 53 integration tests across 12 binaries; WASM compilation passed. The full rational torus passes 26,796 chart-pair obligations and produces one physical face, two loop edges and one seam vertex. Annulus holes, curved irrational cuts, multiple UV patches joined through a seam, empty material and refusal of double coverage are tested. Cached ownership tampering is rejected.

This is an exact native representation. Legacy Model conversion, STEP export, tessellation and viewer integration have not been qualified by these checks. Historical full suites and the prior four kernel tests were not rerun for this snapshot.

Remaining NURBS0173 work: partial/common curved overlaps with original contributor ownership, mixed shore subdivision and source/chart cuts; trim-local geometry qualification. Remaining NURBS0030 work: a general automatic complete surface intersection atlas and actual branch/trim topology. Do not promote helper APIs or focused examples to catalog completion.
