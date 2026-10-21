# Original monotone graph against a containing rectangular plane

The surface/surface report now admits `CoverageProof::MonotoneGraphInRectangle`. This is geometric-set coverage, not branch/trim ownership. NURBS-0030 remains planned.

The second support must be an original axis-aligned rectangular affine chart with constant positive weights. The entire positive rational control hull of the first support must project inside that rectangle in both plane coordinates. Any possible clipping disables the reduction. Periodic source charts are refused.

The original span jets independently prove source regularity and a globally one-signed derivative of the plane residual along one source parameter axis. Strict opposite residual signs on both natural boundaries prove exactly one interior root for every value of the other parameter. Original basis continuity and regularity give one complete transverse graph. This proves that no additional branch can be missed within the source chart, and the rectangle-inclusion proof covers the whole graph on the second chart. The proof uses the same original-source kernel as parallel sections, with an independent bounded 100000-query allowance. Unsupported, exhausted or inconclusive reductions return to the unqualified general solver.

Endpoint root bands are refined by outward interval Newton. Their midpoints and the corresponding original-source evaluations are numerical representatives returned as a `Continued` candidate. Those representatives, the sparse samples and the coedge placeholder do not acquire exact-curve or owned-topology authority. `topology_proven` remains false. Global geometric enclosure is the full original source control hull.

The finite multispan Boolean author retains its independent complete-SS prerequisite and sealed BranchGraph checks. Its graph/slab cases now receive an actual original-source coverage receipt instead of encountering an unsupported general reduction. The existing independent region, volume and rigid-transform naming regressions pass without removing any admission gate.

Core tests cover a nonconstant polynomial root with an independent analytic oracle, a rational original source, and a plane clipping the graph that must remain unproved. The previous seven SS-coverage and five parallel-section regressions also pass. General nonlinear plane charts, clipped graph branches, arbitrary multi-component intersections and topology reconstruction remain separate.
