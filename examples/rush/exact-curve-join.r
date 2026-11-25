// @rush/1
path = nurbs_curve(degree:1,knots:[0,0,1,3,3],control_points:[[0mm,0mm,0mm],[1mm,2mm,0mm],[5mm,4mm,0mm]],weights:[1,2,1])
left = path.curve_split(parameter:1,part:0)
right = path.curve_split(parameter:1,part:1)
joint = left.merge_curve_exact(target_geometry:right)
show joint.surface_extrude(vector:[0,0,1mm]).tessellate(12,4)
