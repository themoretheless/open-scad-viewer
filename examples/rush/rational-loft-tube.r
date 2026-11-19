// @rush/1
a = circle_curve(center:[0,0,0],normal:[0,0,1],radius:5mm)
b = circle_curve(center:[0,0,12mm],normal:[0,0,1],radius:5mm)
body = brep_rational_loft(sections:[[[a]],[[b]]])
show body.brep_tessellate(24)
