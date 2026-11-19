// @rush/1
edge = bezier_curve(points: [[0mm,0mm,0mm],[10mm,0mm,0mm]])
profile = edge.curve_edit(insert_knots:[{value:0.5,count:1}],degree:2,trim:[0.1,0.9],reverse:true)
strip = profile.surface_extrude(vector:[0mm,8mm,0mm])
panel = strip.surface_edit(insert_knots_v:[{value:0.5,count:1}],degree_v:2,trim:[0.2,0.8,0.25,0.75],reverse_v:true)
show panel.tessellate(4,4)
