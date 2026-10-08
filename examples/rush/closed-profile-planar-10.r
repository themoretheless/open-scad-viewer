// @rush/1
// Rust proves the actual retained body. Station joins remain C0; ideal-family error is separate.
p0=bezier_curve(points:[[2.875mm,0mm,-0.125mm],[2.875mm,0mm,0.125mm]])
p1=bezier_curve(points:[[2.875mm,0mm,0.125mm],[3.125mm,0mm,0.125mm]])
p2=bezier_curve(points:[[3.125mm,0mm,0.125mm],[3.125mm,0mm,-0.125mm]])
p3=bezier_curve(points:[[3.125mm,0mm,-0.125mm],[2.875mm,0mm,-0.125mm]])
path=nurbs_curve(degree:2,knots:[0, 0, 0, 0.25, 0.25, 0.5, 0.5, 0.75, 0.75, 1, 1, 1],control_points:[[3mm,0mm,0mm],[3mm,3mm,0mm],[0mm,3mm,0mm],[-3mm,3mm,0mm],[-3mm,0mm,0mm],[-3mm,-3mm,0mm],[0mm,-3mm,0mm],[3mm,-3mm,0mm],[3mm,0mm,0mm]],weights:[1, 1, 2, 2, 4, 4, 8, 8, 16])
show brep_progressive_sweep([[p0,p1,p2,p3]],path,
 scale:{degree:1,knots:[0,0,1,1],values:[1,1],weights:[1,1]},
 twist:{degree:1,knots:[0,0,1,1],values:[0deg,0deg],weights:[1,1]},normal:[0,0,1],orientation:"rmf",
 initial_sections:10,max_sections:10,max_deviation:1mm
).brep_tessellate(4)
