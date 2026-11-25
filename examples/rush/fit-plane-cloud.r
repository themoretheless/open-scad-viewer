// @rush/1
cloud_fit = fit_plane_cloud(points:[
  [1mm,1mm,5mm],
  [3mm,1mm,5mm],
  [3mm,3mm,5mm],
  [1mm,3mm,5mm]
],width:4mm,height:2mm)
show cloud_fit.tessellate(12,16)
