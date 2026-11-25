// @rush/1
line = line_curve(start:[0mm,0mm,0mm],end:[1mm,0mm,0mm])
line_audit = line.affine_control_support(max_work:1000000)
sheet = line_audit.surface_extrude(vector:[0mm,1mm,0mm])
plane_audit = sheet.affine_control_support(max_work:1000000)
trace = nurbs_curve(degree:1,knots:[0,0,1,1],control_points:[[0,0],[1,0]],weights:[1,1])
result = plane_audit.surface_trace_agreement(inputs:[line,trace],reverse:false,tolerance_mm:0.05mm,max_cells:1000)
show result.tessellate(8,8)
