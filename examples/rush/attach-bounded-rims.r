// @rush/1
mesh = triangle_mesh([[0,0,0],[1,0,0],[0,1,0],[0,0,1]],[[0,1,3],[1,2,3],[2,0,3]])
open_body = mesh.mesh_to_nurbs_brep()
cap = open_body.attach_bounded_rims(shell:0,plane_normal:[0,0,-1],plane_offset_mm:0,rim_addresses:[[[0,0],[1,0],[2,0]]],support_tolerance_mm:0.000000001,max_work:1000000,max_verification_cells:10000)
show cap.brep_tessellate(4)
