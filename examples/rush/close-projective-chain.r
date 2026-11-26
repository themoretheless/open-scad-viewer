// @rush/1
a = nurbs_curve(degree:3,knots:[0, 0, 0, 0, 1, 1, 1, 1],control_points:[[4mm,-4mm,0mm],[6mm,-2mm,0mm],[6mm,2mm,0mm],[4mm,4mm,0mm]],weights:[1, 1, 1, 1])
b = nurbs_curve(degree:3,knots:[10, 10, 10, 10, 11, 11, 11, 11],control_points:[[4mm,4mm,0mm],[2mm,6mm,0mm],[-2mm,6mm,0mm],[-4mm,4mm,0mm]],weights:[1, 1, 1, 1])
c = nurbs_curve(degree:3,knots:[20, 20, 20, 20, 21, 21, 21, 21],control_points:[[-4mm,4mm,0mm],[-6mm,2mm,0mm],[-6mm,-2mm,0mm],[-4mm,-4mm,0mm]],weights:[1, 1, 1, 1])
d = nurbs_curve(degree:3,knots:[30, 30, 30, 30, 31, 31, 31, 31],control_points:[[-4mm,-4mm,0mm],[-2mm,-6mm,0mm],[2mm,-6mm,0mm],[4mm,-4mm,0mm]],weights:[1, 1, 1, 1])
result = a.close_projective_chain(inputs:[b,c,d],max_work:1000000)
show result.surface_extrude(vector:[0,0,1mm]).tessellate(32,4)
