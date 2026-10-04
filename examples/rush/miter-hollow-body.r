// @rush/1
outer = circle_curve(center: [0,0,0],normal: [0,0,1],radius: 0.5mm)
hole = circle_curve(center: [0,0,0],normal: [0,0,-1],radius: 0.2mm)
show brep_miter_sweep([[outer],[hole]],
 points: [[0,0,0],[0,0,10mm],[10mm,0,10mm],[10mm,10mm,15mm]],
 normal: [1,0,0],miter_limit: 2
).brep_tessellate(4)
