// @rush/1
source = nurbs_curve(degree:2,knots:[2,2,2,5,5,5],control_points:[[0,0,0],[3mm,6mm,0],[6mm,0,0]],weights:[1,1,1])
left = source.curve_split(parameter:3.5,part:0)
right = source.curve_split(parameter:3.5,part:1)
result = left.compact_join_seam(target_geometry:right,max_work:1000000)
show result.surface_extrude(vector:[0,0,1mm]).tessellate(12,4)
