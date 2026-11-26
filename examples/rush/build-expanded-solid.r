// @rush/1
mesh = triangle_mesh([[0,0,0],[1,0,0],[0,1,0],[0,0,1]],[[0,1,3],[1,2,3],[2,0,3]])
wall = mesh.mesh_to_nurbs_brep()
solid = wall.build_expanded_solid(volume_tolerance_mm3:0.0001,tolerance_uv:0.00000001,max_verification_cells:10000,max_volume_cells:10000,max_work:1000000)
show solid.brep_tessellate(4)
