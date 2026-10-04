// @rush/1
profile = bezier_curve(points: [[3mm,0mm,0mm],[6mm,2mm,0mm],[8mm,0mm,0mm]])
path = bezier_curve(points: [[0mm,0mm,0mm],[0mm,0mm,15mm],[5mm,0mm,30mm]])
show twist_sweep(profile,path,origin: [0mm,0mm,0mm],axis: [0,0,1],start_degrees: 0deg,sweep_degrees: 180deg).tessellate(segments_u: 16,segments_v: 32)
