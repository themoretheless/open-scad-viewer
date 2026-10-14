// @rush/1
shape = catenoid_surface(center: [0mm,0mm,0mm],scale: 2mm,start_z: -2mm,end_z: 3mm,max_deviation: 0.0001mm)
show shape.tessellate(segments_u: 32,segments_v: 32)
