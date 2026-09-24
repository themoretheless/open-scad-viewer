# Own-Rust CAD oracle v3 review

V2 remains byte-immutable. V3 re-captures the five fixed own-Rust direct-evaluator cases against the rebuilt geometry kernel after the committed optimization series (BVH convex-parts, borrowing combine, release-profile tuning).

Scope of the migration: `boolean-difference`, `repeated-loop-identities`, `reduced-preview`, and `reduced-preview-full-companion` change only their LME1/LSE1 byte-level mesh/scene hashes — the f32 kernel cannot match the f64 oracle byte-wise. Every frozen semantic requirement still holds: volumes and surface areas within the predeclared 1e-9 tolerance, identical record sizes, topology, provenance, colors, identities, and closed consistently oriented incidence. `colored-transform` is byte-identical to v2 and is not migrated.

The capture ran twice in fresh Node processes with identical output; the kernel was rebuilt immediately before capture and no source or artifact changed during recording. No qualification claim is made; this oracle remains a deterministic differential baseline, not Manifold binary compatibility or whole-application qualification.
