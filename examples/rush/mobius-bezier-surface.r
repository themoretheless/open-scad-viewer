// @rush/1
source = bezier_surface(points:[[[0,0,0],[0,2mm,0]],[[1mm,0,0],[1mm,2mm,0]]])
result = source.mobius_bezier_surface(clock_factor:[2,0.5],max_work:1000000)
show result.tessellate(8,8)
