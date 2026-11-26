// @rush/1
source = nurbs_curve(degree:2,knots:[-2, -1, 0, 1, 3, 4, 5, 6, 8],control_points:[[0mm,0mm,0mm],[2mm,0mm,0mm],[2mm,1mm,0mm],[0mm,2mm,0mm],[0mm,0mm,0mm],[2mm,0mm,0mm]],weights:[1, 2, 1, 4, 1, 2],periodic:true)
result = source.rotate_periodic_knot(parameter:3,max_work:1000000)
show result.surface_extrude(vector:[0,0,1mm]).tessellate(32,4)
