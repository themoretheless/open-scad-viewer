// @rush/1
rack = brep_rack(teeth:12,module:2mm,pressure_angle:20,backlash:0.1mm,addendum:2mm,dedendum:2.5mm,backing_height:3mm,z_min:-1mm,z_max:4mm)
show rack.brep_tessellate(2)
