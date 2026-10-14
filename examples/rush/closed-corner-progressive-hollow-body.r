// @rush/1
outer = circle_curve(center: [0,8mm,0],normal: [0,-1,0],radius: 0.25mm)
hole = circle_curve(center: [0,8mm,0],normal: [0,1,0],radius: 0.1mm)
path = round_polyline_curve(points: [[0,0,0],[10mm,0,0],[10mm,10mm,0],[0,10mm,0]],radius: 2mm,closed: true)
show brep_progressive_sweep([[outer],[hole]],path,
 scale: {degree: 1,knots: [0,0,1,1],values: [1,1],weights: [1,1]},
 twist: {degree: 1,knots: [0,0,1,1],values: [0deg,0deg],weights: [1,1]},
 normal: [0,0,1],initial_sections: 5,max_sections: 257,max_deviation: 0.02mm
).brep_tessellate(2)
