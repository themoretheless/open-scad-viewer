# Closed original material and constructor-owned Solid

`brep-core::sweep_boundary_owned::construct_closed_with_ideal_atlas` composes the original-volume atlas with a constructor-owned periodic BRep. The material `LoftCap` is a reference filled section with UV contours, not a physical end cap. The result proves the original body's regularity/embedding/parameter topology, a continuous boundary approximation bound, and independent geometric validity of the constructed Solid. It does not claim an ambient isotopy between source and retained bodies or any G1/G2 upgrade.

## Original material ownership

Every original profile and UV curve is validated. Exact coefficient decomposition covers multispan world and UV chains, including one closed UV curve. Rounded decomposition does not qualify original material ownership. The UV audit proves exact joins, simple outer/hole loops, separation, winding and containment. Reversing every loop preserves material roles; same-winding or touching/nested holes remain refused.

Every original world span must equal the composition of its original UV span with the same original carrier. `curve_surface_agreement::verify_exact` also requires chart-domain inclusion; algebraic extrapolation outside the source rectangle cannot supply a material certificate. The UV and world source definitions remain immutable. A proof limitation or insufficient work returns no body.

The original atlas then covers the entire filled carrier and its exact S1 quotient. Since the material UV region and all its contours are now owned, the original regularity/embedding/topology result no longer relies on a caller's declaration that this region is valid. This still belongs to the existing sufficient atlas family: one nonperiodic carrier chart, exact source seam ownership and admitted original frame policies. Pole quotients, periodic-storage paths and arbitrary multichart carriers remain outside that family.

## Actual retained boundary

The constructor obtains one selected level and its original source-wall bound. It builds the actual station curves from that same immutable `MultiSweep`, bounds the retained curve decomposition, and requires the same positive rational basis across stations before propagating the curve bound over each ruled wall. The whole closed boundary bound is the original wall bound plus this decomposition bound. No endpoint cap term is introduced.

The native periodic material builder supplies only topology and an orientation proposal. Its initial section may be nonplanar. The control-polygon area vector does not certify a normal, planarity, material validity or outward orientation. Existing planar/capped builder behavior remains unchanged. The new owner independently verifies complete retained coefficient-family and rectangular-domain coverage, all face regularity, exact boundary agreement, every face and face pair, shell nesting/material roles and actual orientation. The volume verifier receives an empty physical-cap selection. Exact sweep hull-range certificates may classify whole disjoint ranges; unvisited pairs never receive credit.

`ClosedBody` has private fields and exposes an immutable model/report. No caller-supplied model, certificate or allowed-contact list can enter construction. A failed stage returns `ClosedAttempt::Refused` with its partial diagnostics and no model. Malformed definitions/limits are errors. A nonclosed source is a semantic refusal; near closure never replaces exact original endpoint identity.

## Limits and provenance

Source cells are shared by original profile regularity and the complete original wall proof. Source products cover exact UV/world decomposition, charged station-control copies and retained decomposition certification. Original contour binding and retained wall-domain ownership share one exact-work ledger; an opaque inconclusive exact composition conservatively reserves the remaining allowance and is reported as unproved, not a demonstrated mismatch or collision. The original-volume atlas and material/retained-volume audits have their own explicit stage budgets and retain their reports. Zero positive-stage budgets are refused before invoking helpers that require positive limits.

The constructor allows 4..1025 selected stations subject to the source policy's own limits, 1..16 loops, the `MultiSweep`'s bounded profiles, at most 256 original UV curves/exact UV pieces, and at most 1024 retained faces. The original atlas may request up to one million aggregate cells while primitive source stages retain their existing ceilings. Insufficient work never produces a partially certified Solid.

## Native protocol and qualification

Rust/JSON operation `brep_sweep_closed_material_construct` uses the same original source-law parser as the open constructor. It requires `material`, `loopSizes`, selected `sections`, deviation/UV tolerance, `idealBody` atlas limits, and `limits` with material and wall-face budgets. The Rust owner computes every flag. Output scope is `original-closed-sweep-law-and-material-region`, `physicalCapCount` is zero, and cap error is null. `smoothnessCertified` remains false and general seam continuity is explicitly C0. Input success flags and a substituted model cannot override any native stage.

The native matrix includes polynomial/rational spatial paths, rational nonplanar carriers, independent domains, variable scale/twist and affine axes/center, parameter/arc spacing, nonplanar material with a hole, spatial authored/guide frames, reversed roles, exact multispan chains, and short/zero resources. The same 0.05 error budget is retained. Parameter examples use 33 stations where admitted; conservative arc or guided bounds may require 129. Coarse rejection is preserved rather than relaxing the requested deviation or altering the source geometry.

The public native JSON fixture is generated by the successful Rust spatial construction. JSON tests cover success, caller forgery, invalid source data and incomplete proof stages. Direct native STEP export/import checks geometry and topology and recomputes retained-volume validity on the restored model. These are not independent OCCT, WASM, browser or CI qualification.

See `docs/qualification/closed-material-body-2026-10-16/`. The successor checks changed BRep/bridge owners, JSON/STEP and no-codec libraries fresh. Only two lower NURBS checks may be inherited after proving every lower source/test/dependency, manifest, lockfile/build configuration and toolchain input byte-identical. Earlier source and qualification checkpoints remain immutable.

Remaining full-goal work includes applicable geometric G1/G2 on actual constructed walls, filled cap/wall joins and closed seams, with original-to-retained correspondence and independent complete-boundary proof. The native station-smoothing candidates and full-boundary inspectors remain useful building blocks; source frame C2 alone supplies no retained G2 guarantee. Sharp joins stay C0. General original frame policies, cap/pole atlases, fresh main integration and delivery qualification remain open.
