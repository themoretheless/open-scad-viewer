# brep-topology

Independent Rust boundary topology. Depends only on serde, not on either geometry kernel.

`Model<C,S,P>` stores vertices, edges, oriented edge uses (coedges), ordered loops, faces with outer/hole loops, oriented shell face uses, and bodies with outer/cavity shells. Array indices are model-local IDs, preserved across serialization and tessellation; callers must remap references when deleting or reordering entities. Geometry payload types C/S/P are supplied by adapters: rational curves/surfaces for NURBS; face meshes and unit edge/pcurve slots for polygons.

`validate_topology()` checks reference integrity, ownership, loop closure, shell connectivity, edge incidence/orientation, connected vertex fans and closed body boundaries. Open sheet shells are supported. It does not establish geometric containment, absence of self-intersection, outward orientation in physical space or printability. A topologically valid shell is not proof of a valid geometric solid.

Limits: 4096 entities, 256 faces, 8192 coedges; coordinate/tolerance bounds. Exact NURBS trimming, sewing arbitrary incompatible boundaries, geometric B-rep booleans and STEP are not implemented.
