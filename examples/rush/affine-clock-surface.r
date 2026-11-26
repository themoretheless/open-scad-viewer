// @rush/1
source = nurbs_surface(degree_u:1,degree_v:1,knots_u:[0,0,2,3,3],knots_v:[0,0,1,3,3],control_points:[[[0,0,0],[0,1mm,0],[0,3mm,0]],[[2mm,0,0],[2mm,1mm,1mm],[2mm,3mm,2mm]],[[3mm,0,0],[3mm,1mm,1mm],[3mm,3mm,2mm]]],weights:[[1,1,1],[1,1,1],[1,1,1]])
result = source.affine_clock_surface(clock_pairs:[[6,0],[-3,3]],max_work:1000000)
show result.tessellate(8,8)
