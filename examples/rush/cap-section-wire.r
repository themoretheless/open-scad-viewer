// @rush/1
outer = nurbs_curve(degree:1,knots:[0,0,1,2,3,4,4],control_points:[[0,0,2],[4,0,2],[4,4,2],[0,4,2],[0,0,2]],weights:[1,1,1,1,1])
cap = outer.cap_section_wire(inputs:[],plane_normal:[0,0,1],plane_offset_mm:2,boundary_tolerance_mm:0,support_tolerance_mm:0,max_work:1000000)
show cap.tessellate(16,16)
