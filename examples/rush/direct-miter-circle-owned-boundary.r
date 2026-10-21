// @rush/1
outer = circle_curve(center: [0,0,0],normal: [1,0,0],radius: 0.5mm)
hole = circle_curve(center: [0,0,0],normal: [-1,0,0],radius: 0.2mm)
show brep_miter_sweep([[outer],[hole]],
 points: [[0,0,0],[10mm,0,0],[13mm,9mm,0.000000000004mm],[-2mm,11mm,0.000000000001mm],[1mm,5mm,-0.000000000002mm]],
 normal: [0,0,1],closed: true,miter_limit: 4,max_deviation: 0.01mm,
 circle_correction_tolerance: 0.000000001mm,circle_correction_quantum: 0.0000000000009094947017729282mm,circle_correction_max_work: 1000000
).brep_tessellate(4)
