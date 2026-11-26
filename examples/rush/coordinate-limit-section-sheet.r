// @rush/1
edge0 = nurbs_curve(degree:1,knots:[0,0,1,1],control_points:[[0, 999997.75, 2],[2, 999995.75, 2]],weights:[1,1])
edge1 = nurbs_curve(degree:1,knots:[0,0,1,1],control_points:[[2, 999995.75, 2],[3, 999997.75, 3]],weights:[1,1])
edge2 = nurbs_curve(degree:1,knots:[0,0,1,1],control_points:[[3, 999997.75, 3],[1, 999999.75, 3]],weights:[1,1])
edge3 = nurbs_curve(degree:1,knots:[0,0,1,1],control_points:[[1, 999999.75, 3],[0, 999997.75, 2]],weights:[1,1])
sheet = edge0.expanded_section_sheets(inputs:[edge1,edge2,edge3],loops:[],plane_normal:[-1,-1,3],plane_offset_mm:-999991.75,tolerance_mm:0.00000001,max_verification_cells:10000,max_work:1000000)
show sheet.brep_tessellate(4)
