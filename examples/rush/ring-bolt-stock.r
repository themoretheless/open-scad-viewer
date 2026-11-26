// @rush/1
gear = brep_gear(module:1.5mm,teeth:40,internal:true,rim_width:6.4mm,height:2mm,clearance:0.1mm)
result = gear.ring_bolt_stock(outer_radius_mm:38mm,bolt_radius_mm:35mm,hole_diameter_mm:4mm,hole_count:6)
show result.brep_tessellate(3)
