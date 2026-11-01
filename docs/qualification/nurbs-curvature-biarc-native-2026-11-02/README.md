# Native curvature reconstruction and finite biarc checks

NURBS-0217 and NURBS-0218 qualify two existing APIs after fixing their numerical contracts. These are two capabilities, not separate items per sample type, curve orientation or continuity label.

The old curvature reconstruction fails both independent regressions: a straight nonuniform four-station input reports a spurious 0.02 quadrature estimate, and a 33-station clothoid endpoint differs from independent dense Fresnel integration by about 8e-5. The fixed path integrates the piecewise-linear curvature model and uses a shared 4096-panel limit. Three-site fitting, explicit data-site convergence and undefined curvature diagnostics are recorded.

The old biarc path accepts a short arc for a 450-degree circle traversal, omitting the source point (-1,0) by sqrt(2) despite near-zero radial error. The fixed path compares finite arcs/segments and their endpoints. This remains sampled one-way geometry, not a Hausdorff or winding certificate.

All 66 public integration tests pass, including 13 new tests and the previous 53, without skips or filters. Library checking without default features passes. Receipt hashes bind actual source, Cargo.lock and nine test binaries. This does not claim a full current workspace suite, application integration, hosted CI or main delivery.

Read the design documents for numerical and refusal boundaries. Original failing logs are preserved.
