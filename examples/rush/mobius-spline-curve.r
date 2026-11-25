// @rush/1
source = nurbs_curve(degree:1,knots:[0,0,2,3,3],control_points:[[0,0,0],[2mm,1mm,0],[3mm,0,0]],weights:[1,1,1])
result = source.mobius_spline_curve(clock_factor:2,max_work:1000000)
show result.surface_extrude(vector:[0,0,1mm]).tessellate(8,4)
