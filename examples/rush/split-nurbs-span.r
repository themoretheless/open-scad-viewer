// @rush/1
path = nurbs_curve(degree:1,knots:[0,0,1,3,3],control_points:[[0mm,0mm,0mm],[1mm,2mm,0mm],[5mm,4mm,0mm]],weights:[1,2,1])
span = path.curve_bezier_span(index:1)
half = span.curve_split(parameter:2,part:1)
show half.surface_extrude(vector:[0mm,0mm,1mm]).tessellate(4,4)
