// @rush/1
mesh = triangle_mesh([[0,0,0],[1,0,0],[0,1,0],[0,0,1],[1,0,1],[0,1,1]],[[0,1,4],[0,4,3],[1,2,5],[1,5,4],[2,0,3],[2,3,5]])
wall = mesh.mesh_to_nurbs_brep()
solid = wall.close_bounded_planes(shell:0,planes:[[0,0,-1,0,0,0],[0,0,1,1,0,0]],max_work:1000000,max_verification_cells:10000)
show solid.brep_tessellate(4)
