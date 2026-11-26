// @rush/1
source = nurbs_curve(degree:3,knots:[0, 0, 0, 0, 1, 1, 1, 2, 2, 2, 3, 3, 3, 4, 4, 4, 4],control_points:[[4mm,-4mm,0mm],[6mm,-2mm,0mm],[6mm,2mm,0mm],[4mm,4mm,0mm],[2mm,6mm,0mm],[-2mm,6mm,0mm],[-4mm,4mm,0mm],[-6mm,2mm,0mm],[-6mm,-2mm,0mm],[-4mm,-4mm,0mm],[-2mm,-6mm,0mm],[2mm,-6mm,0mm],[4mm,-4mm,0mm]],weights:[1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1],periodic:false)
result = source.store_periodic_curve(max_work:1000000)
show result.surface_extrude(vector:[0,0,1mm]).tessellate(32,4)
