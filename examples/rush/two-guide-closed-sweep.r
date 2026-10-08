// @rush/1
profile = circle_curve(center: [1mm,0mm,0mm],normal: [0,0,1],radius: 1mm)
a = circle_curve(center: [0mm,0mm,0mm],normal: [0,0,1],radius: 10mm)
b = circle_curve(center: [0mm,0mm,0mm],normal: [0,0,1],radius: 12mm)
show two_guide_sweep(profile,a,b,width: 2mm,axis_y: [0,0,1],axis_z: [0,1,0]).tessellate(segments_u: 32,segments_v: 32)
