// @rush/1
left = nurbs_curve(degree:2,knots:[0,0,0,1,1,1],control_points:[[-2mm,4mm,0mm],[-1mm,0mm,0mm],[0mm,0mm,0mm]],weights:[1,1,1])
right = nurbs_curve(degree:2,knots:[0,0,0,1,1,1],control_points:[[0mm,0mm,0mm],[0.5mm,0mm,0mm],[1mm,1mm,0mm]],weights:[1,1,1])
join = left.curve_join_continuity(target_geometry:right,source_end:"end",target_end:"start",tolerance_mm:0.0001mm,tangent_tolerance:0.0001,curvature_tolerance_per_mm:0.0001/1mm)
show join.surface_extrude(vector:[0mm,0mm,1mm]).tessellate(16,4)
