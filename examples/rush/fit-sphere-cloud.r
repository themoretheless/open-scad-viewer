// @rush/1
cloud_fit = fit_sphere_cloud(points:[
  [2mm,0mm,0mm],
  [-2mm,0mm,0mm],
  [0mm,2mm,0mm],
  [0mm,-2mm,0mm],
  [0mm,0mm,2mm],
  [0mm,0mm,-2mm]
])
show cloud_fit.tessellate(12,16)
