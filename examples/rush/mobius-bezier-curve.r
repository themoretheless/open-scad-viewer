// @rush/1
source = nurbs_curve(degree:1,knots:[2,2,5,5],control_points:[[0,0,0],[1mm,2mm,0]],weights:[1,2])
result = source.mobius_bezier_curve(clock_factor:2,max_work:1000000)
show result.surface_extrude(vector:[0,0,1mm]).tessellate(8,4)
