// @modelgraph-text/1
// Replace triangle_mesh data with a source mesh, or use a mesh from any own kernel.
mesh = brep_box([-10,-10,-10],[10,10,10]).brep_tessellate(1)
show mesh.mesh_to_sdf().sdf_offset(1mm).sdf_tessellate([-15,-15,-15],[15,15,15],[12,12,12])
