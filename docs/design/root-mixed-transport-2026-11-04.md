# Lossless root/mixed BRep transport

New finite face split/region/retention operations produce `RootBrep`, while existing sewing, topology transfer and qualified root Boolean infrastructure uses `MixedBrep`. `MixedBrep::from_root` now transports those results directly into mixed storage. `MixedBrep::to_root` recovers pure-root storage when every vertex, world edge and pcurve has original-root authority.

Every topology index, surface, shell/body reference, coedge traversal and projection support is retained. World curves and global algebraic vertex selectors keep their literal `Arc` owners. No poles are fitted and no clock is replaced. Rational or mixed rational payloads refuse pure-root recovery. Root endpoint ranges remain owned by ports; foreign trim payloads cannot be silently dropped on recovery.

Conversion grants storage authority only. It does not promote an open trimmed patch to closed material or bypass the existing solid/Boolean qualification protocol. Geometry bridge/WASM command exposure remains separate work.

Tests check full paired-model roundtrip, literal vertex/graph identity, edge and pcurve indices, preserved directions and incidence; the mixed-pair split with a retained hole also survives roundtrip with UV closure and hole classification. Existing mixed rational models refuse pure-root recovery.

Full remaining scope includes general singular/tangent/periodic/pole SS branches, complete arrangements/Jordan/nesting and automatic retained material, shell orientation/cavities/healing, unrestricted intersecting Boolean and actual bridge/WASM calls. This conversion connects native storage pipelines; it does not close those proof requirements.

Validation: 13 distinct tests passed across root_model, root_uv_closure and mixed_topology_transfer. A real MixedBrep::concatenate call roundtrips into RootBrep. Architecture audit passed: 54 pinned oversized files, zero layer violations. Logs: docs/qualification/root-mixed-transport-2026-11-04.
