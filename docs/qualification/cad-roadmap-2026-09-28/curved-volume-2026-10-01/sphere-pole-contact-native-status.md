# Exact sphere pole contact stage

The exact hull-contact certificate now restricts controls to simultaneous supporting planes before intersecting their hulls. For positive rational weights, a point on an extreme plane can only use controls on that plane. A plane through a hull interior is excluded from this rule. All extrema and equality checks use the original authored binary64 coordinates; no rounded fitted plane is introduced.

This proves that sphere face pairs `[0,2]`, `[1,3]`, `[4,6]` and `[5,7]` meet only at their common authored pole. A point contact still requires the same topological vertex and exact incident edge endpoints. Duplicating the topology while retaining all coordinates refuses the certificate. A control hull straddling the candidate plane also refuses the point-contact claim.

The enclosing audit admits these certificates only after exact edge/lift agreement, exact trim joins, valid simple trims and global chart injectivity. Sphere radius 3 passes those prerequisites with a 1000-section face budget and obtains the four pole certificates. Its remaining distinct-face contacts are unresolved; the regression requires `proven=false` and `absenceProven=false` and checks input immutability.

All 39 B-rep tests selected by `boundary_` pass, covering the new four hull-contact tests, aggregate boundary embedding, exact agreement and existing boundary/distance regressions. This stage is native; the live WASM build was started before this hull refinement and does not include it. General sphere volume validity and hemisphere/equator contact classification remain open.
