# Exact owned gear profile joins

`brep_core::gear::profile_with_report` returns the original planar candidate profile and geometry report. It is also the profile owner used by `gear_with_report`; the solid builder no longer recreates a separate profile. Every neighboring curve endpoint, including each full-wire closing seam and hole seam, now has an identical saved coordinate.

The canonical endpoint comes from the certified quintic flank if one meets the junction; otherwise the following curve owns it. Every flank pole and its original basis/weights remain unchanged. Distinct adjacent flank endpoints cannot be reconciled by moving a certified flank. Only neighboring line/arc endpoints move.

The report's `profile_join_adjustment_upper` bounds the complete change from the previous line/arc construction. Original positive rational basis functions form a partition of unity, so maximum changed-pole displacement bounds every curve point. Outward L1 displacement encloses Euclidean displacement without a trusted hypot/sqrt evaluation. Adjustments over 1e-7 are refused. This is a continuous change bound, not nominal circle/root accuracy or embedded-solid qualification.

Internal-hole reversal is checked against the actual final reverse knots. Both the stored-parameter flank proof and nominal parameter-box proof are recomputed for the reversed definition; a real-arithmetic assumption that knot subtraction is exact is not used.

Two integration tests cover exact complete-wire joins for 3/8/20-tooth external profiles, a bore and a 40-tooth internal profile, and independently checked exact original-profile certificates for 3 and 8 teeth. The exact certificate owner reconstructs every original rational knot span and checks all span and pair witnesses, orientation and nesting. It is distinct from closed incidence and sampled polygons. A focused native unit test independently checks every unchanged quintic pole, unchanged weights/knots, whole-curve movement samples for rational parameterizations and an insufficient adjustment budget.

The exact profile owner currently limits original spans to 128 and subdivided pieces to 256. The small-gear certificates are not a claim of qualification for every supported tooth count or internal ring. Nominal root-transition accuracy, scalable whole-profile proof, and full solid geometric admission remain separate. Catalog total remains 189/200.
