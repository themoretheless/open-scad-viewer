// @rush/1
panel = bezier_surface(points:[[[0,0,0],[0,2mm,0]],[[1mm,0,0],[1mm,2mm,0]]])
result = panel.projective_surface_exact(coefficients:[[1,0,0,0],[0,1,0,0],[0,0,1,0],[1,0,0,1]],max_work:1000000)
show result.tessellate(8,8)
