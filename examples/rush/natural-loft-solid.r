// @rush/1
a = circle_curve(center: [0mm,0mm,0mm],normal: [0,0,1],radius: 10mm)
b = circle_curve(center: [0mm,0mm,20mm],normal: [0,0,1],radius: 15mm)
c = circle_curve(center: [0mm,0mm,40mm],normal: [0,0,1],radius: 10mm)
show brep_natural_loft(sections: [[[a]],[[b]],[[c]]],parameters: [0,0.5,1]).brep_tessellate(segments: 16)
