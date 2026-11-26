// @rush/1
surface = fit_surface_cloud(points:[[0mm,0mm,5mm],[2mm,0mm,5mm],[0mm,3mm,5mm],[2mm,3mm,5mm],[1mm,1mm,5mm],[1mm,2mm,5mm]],controls_u:2,controls_v:2)
show surface.tessellate(8,8)
