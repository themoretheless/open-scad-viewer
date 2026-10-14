// @rush/1
profile = bezier_curve(points: [[0mm,0mm,0mm],[5mm,3mm,0mm],[10mm,0mm,0mm]])
a = line_curve(start: [0mm,0mm,0mm],end: [0mm,0mm,25mm])
b = bezier_curve(points: [[10mm,0mm,0mm],[16mm,0mm,12mm],[12mm,0mm,30mm]],weights: [1,2,1])
show two_guide_sweep(profile,a,b,width: 10mm,axis_y: [0,1,0],axis_z: [0,0,1]).tessellate(segments_u: 16,segments_v: 16)
