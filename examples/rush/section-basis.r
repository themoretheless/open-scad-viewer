// @rush/1
anchor = line_curve(start:[0mm,0mm,0mm],end:[2mm,0mm,0mm])
back = nurbs_curve(degree:2,knots:[0,0,0,0.5,1,1,1],control_points:[[2mm,1mm,0],[1.5mm,1mm,0],[0.5mm,1mm,0],[0,1mm,0]],weights:[1,1,1,1])
prepared = anchor.compatible_section(inputs:[back],piece:0)
forward = prepared.oriented_section(inputs:[back],piece:1,tolerance_mm:0.001mm)
show forward.surface_extrude(vector:[0,0,1mm]).tessellate(8,8)
