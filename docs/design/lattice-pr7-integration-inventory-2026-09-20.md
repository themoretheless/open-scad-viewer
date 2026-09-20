# PR7 Integration Inventory

Source: 7b3afccf379aead6151c7ceaf9b633e40eb62924. Current main inspected after
2340ba45. This is a feature disposition, not a claim that PR7 is merged.

## Integrated Replacements

| Branch capability | Current implementation / evidence |
| --- | --- |
| BCC/octet/isogrid geometry | Bounded native geometry ports; lattice-strength-integration design record |
| Print dimension fitting | Existing native fit plus explicit nominal opening limit; lattice-opening-fit record |
| Truss displacement/member response | Native bounded axial solver, explicit singularity/refusal and equilibrium checks |
| Forces and moments | Native wrench assembly with resultant checks, explicit node selection and origin |
| Supports and load combinations | Explicit XYZ masks and signed cases; incompatible supports refused, not unioned |
| Workbench calculation and reports | NominalTrussPanel, shared worker, editable cases, member/node tables and JSON |

The UI analyses a nominal bounding-box axial graph, not the clipped finished
solid. This boundary is explicit and tested; it is not an implementation of
the branch's implied skin/core contribution to part strength.

## Do Not Import Unchanged

- Dense solver silently accepting mechanisms and incorrect moment/support
  assembly: reproduced by the existing audit and replaced by checked native
  calculations. Preserve its history, not its numerical behavior.
- Heuristic material/environment/fatigue scalars and fixed material presets:
  no provenance or calibration accompanies these constants in the inspected
  module. Current calculations require explicit E and member area. This does
  not establish that every possible material approximation is invalid.
- Homogenized strength/stiffness, expected percentage design improvements and
  automatic variant winners: these depend on the unvalidated proxy model and
  cannot be presented as verified computed responses.
- Automatic print optimization: the audit proves it can increase the opening
  beyond both the initial value and its requested limit. The nominal fit
  replacement enforces its stated bound or refuses.

The extended read-only audit additionally proves that latticeDesignAdvice
returns "Margins OK" for NaN utilization and buckling ratio. Variant comparison
returns winner b with a NaN stiffness input and NaN delta. The script records
these as explicit invalid-input observations, not JSON null-valued metrics.

## Still Missing

- Geometric printability report with explicit build direction and actual
  geometry measurements. The old orientation ranking has no mesh input and
  uses hardcoded pattern scores; its recommendations are not a verified port.
- Slicer settings export with a declared schema, supported destination and
  validation. The branch explicitly labels its JSON as hints, not a vendor
  profile; this must not become a false import-compatibility claim.
- Spatial result visualization. Current member/node tables and JSON do not
  replace the branch's viewport markers. New markers should represent actual
  solved fields and selected graph geometry, not material-biased weak spots.
- Optional calibrated strength/buckling assessments and variant comparison,
  if retained as product features, need explicit model assumptions and refusal
  behavior before UI integration. No automatic safe-design recommendation.

These are real remaining decisions/work, not reasons to demand a complete
finished-solid certification system before integrating useful PR7 changes.
Do not mark the branch merged or delete its unique history based on this list.

## Verification

`node scripts/audit-lattice-branch.mjs` passes on the pinned unchanged modules,
including all prior moment/support/print-fit observations and the two new
invalid recommendation checks. No production or WASM changes in this audit.
Related evidence: lattice-strength-integration, truss-scenarios,
nominal-truss-workbench and lattice-opening-fit design records.
