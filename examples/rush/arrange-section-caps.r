// @rush/1
outer = nurbs_curve(degree:1,knots:[0,0,1,2,3,4,4],control_points:[[0,0,2],[4,0,2],[4,4,2],[0,4,2],[0,0,2]],weights:[1,1,1,1,1])
hole = nurbs_curve(degree:1,knots:[0,0,1,2,3,4,4],control_points:[[1,1,2],[3,1,2],[3,3,2],[1,3,2],[1,1,2]],weights:[1,1,1,1,1])
island = nurbs_curve(degree:1,knots:[0,0,1,2,3,4,4],control_points:[[1.5,1.5,2],[2.5,1.5,2],[2.5,2.5,2],[1.5,2.5,2],[1.5,1.5,2]],weights:[1,1,1,1,1])
cap = outer.arrange_section_caps(inputs:[],loops:[[hole],[island]],region:1,plane_normal:[0,0,1],plane_offset_mm:2,boundary_tolerance_mm:0,support_tolerance_mm:0,max_work:1000000)
show cap.tessellate(16,16)
