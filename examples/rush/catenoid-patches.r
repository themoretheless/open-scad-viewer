// @rush/1
shape = catenoid_patches(center: [0mm,0mm,0mm],scale: 1mm,start_z: -5mm,end_z: 5mm,max_deviation: 0.0001mm)
show shape.nurbs_patches_tessellate(segments: 12)
