// @rush/1
gear = brep_gear(module:1.5mm,teeth:32,height:2mm,clearance:0.1mm)
result = gear.eccentric_bore_stock(offset_x_mm:3mm,offset_y_mm:-4mm,hole_diameter_mm:10mm,minimum_web_mm:0.375mm)
show result.brep_tessellate(3)
