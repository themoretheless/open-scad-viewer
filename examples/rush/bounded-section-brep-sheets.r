// @rush/1
outer0 = nurbs_curve(degree:1,knots:[0,0,1,1],control_points:[[0,0,0.3333333333333333],[1,0,0.6666666666666666]],weights:[1,1])
outer1 = nurbs_curve(degree:1,knots:[0,0,1,1],control_points:[[1,0,0.6666666666666666],[1,1,1.0]],weights:[1,1])
outer2 = nurbs_curve(degree:1,knots:[0,0,1,1],control_points:[[1,1,1.0],[0,1,0.6666666666666666]],weights:[1,1])
outer3 = nurbs_curve(degree:1,knots:[0,0,1,1],control_points:[[0,1,0.6666666666666666],[0,0,0.3333333333333333]],weights:[1,1])
cap = outer0.bounded_section_brep_sheets(inputs:[outer1,outer2,outer3],loops:[],plane_normal:[-1,-1,3],plane_offset_mm:1,support_tolerance_mm:0.00000000000001,tolerance_mm:0.00000001,max_verification_cells:10000,max_work:1000000)
show cap.brep_tessellate(12)
