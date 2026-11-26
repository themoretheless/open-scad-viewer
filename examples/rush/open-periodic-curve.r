// @rush/1
source = nurbs_curve(degree:2,knots:[-2, -1, 0, 1, 2, 3, 4, 5, 6],control_points:[[0mm,0mm,0mm],[1mm,0mm,0mm],[1mm,1mm,0mm],[0mm,1mm,0mm],[0mm,0mm,0mm],[1mm,0mm,0mm]],weights:[1, 1, 2, 2, 1, 1],periodic:true)
result = source.open_periodic_curve(max_work:1000000)
show result.surface_extrude(vector:[0,0,1mm]).tessellate(32,4)
