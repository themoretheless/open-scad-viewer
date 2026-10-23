# Experimental preview bridge

The Rust bridge accepts `brep_nurbs_partial_annular_preview` with exactly
model, edge and radius fields. It returns the source-bound preview model and
ChangeSet with qualification status `preview-only`, commitAllowed=false,
boundaryIntersectionProof=unqualified and transitionContinuityProof=unqualified.
It does not return a FeatureCertificate or AuditedFeatureResult.

TypeScript exposes a distinct PartialAnnularPreview type and wrapper. The
existing audited edge feature dispatcher remains unchanged: no partial preview
is admitted as an audited document edit. Worker/UI wiring remains to be added.

Native bridge tests verify the explicit preview status, lack of a certificate,
body identity retention and closed topology, invalid edge refusal and unknown
request-field refusal. Rebuilding WASM and browser runtime validation are still
required before claiming this bridge is available in the running application.
