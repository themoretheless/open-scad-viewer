"""Run with Blender --background --factory-startup --python this_file."""
import importlib.util
from pathlib import Path
import bpy
root = Path(__file__).resolve().parent.parent
spec = importlib.util.spec_from_file_location('solid_bridge', root/'integrations/blender/solid_bridge.py')
bridge = importlib.util.module_from_spec(spec); spec.loader.exec_module(bridge)
def payload(width=2):
    return {'schema':bridge.SCHEMA,'units':'mm','projectId':'qualification','bodies':[{
        'id':'body-a','name':'Part','mesh':{'positions':[0,0,0,width,0,0,0,3,0],'indices':[0,1,2]}}]}
external = bpy.data.objects.new('External scene object', None)
bpy.context.scene.collection.objects.link(external)
bridge.update_scene(payload())
obj = next(o for o in bpy.data.objects if o.get(bridge.BODY)=='body-a')
pointer = obj.as_pointer()
obj.location = (4,5,6)
modifier = obj.modifiers.new('User modifier', 'SOLIDIFY');modifier.thickness=.2
material = bpy.data.materials.new('User material');obj.data.materials.append(material)
obj['user_metadata']='keep'
bridge.update_scene(payload(7))
assert obj.as_pointer()==pointer and obj.location[:]==(4.,5.,6.)
assert obj.modifiers[0].name=='User modifier' and obj.data.materials[0]==material
assert obj['user_metadata']=='keep' and obj.data.vertices[1].co.x==7
assert external.name in bpy.data.objects
bad=payload(9);bad['bodies'][0]['mesh']['indices']=[0,1,99]
try: bridge.update_scene(bad)
except ValueError: pass
else: raise AssertionError('Invalid input accepted')
assert obj.data.vertices[1].co.x==7
empty=payload();empty['bodies']=[];bridge.update_scene(empty)
assert obj.get('osv_missing_from_source') and obj.as_pointer()==pointer
bridge.update_scene(payload(8));assert not obj['osv_missing_from_source']
print('BLENDER_BRIDGE_VERIFIED: identity, geometry update, transforms, modifiers, materials, external object, refusal, missing/reappear')

uv = obj.data.uv_layers.new(name='User UV')
try: bridge.update_scene(payload(12))
except ValueError: pass
else: raise AssertionError('UV loss accepted')
assert obj.data.vertices[1].co.x == 8 and obj.data.uv_layers.get('User UV')
bridge.register()
import json, tempfile
obj.data.uv_layers.remove(uv)
with tempfile.TemporaryDirectory() as directory:
    path = Path(directory)/'qualification.osv-blender.json'
    path.write_text(json.dumps(payload(11)))
    assert bpy.ops.import_scene.solid_snapshot(filepath=str(path)) == {'FINISHED'}
    assert obj.data.vertices[1].co.x == 11 and obj.as_pointer() == pointer
bridge.unregister()
print('BLENDER_BRIDGE_ADDON_VERIFIED: registration and UV refusal')

# Malformed containers must fail before any Blender data is created or replaced.
for malformed in [None, [], {'schema':bridge.SCHEMA,'units':'mm','projectId':'x','bodies':[None]},
                  {'schema':bridge.SCHEMA,'units':'mm','projectId':'x','bodies':[{'id':'a','name':'A','mesh':None}]}]:
    counts = (len(bpy.data.objects), len(bpy.data.meshes), len(bpy.data.collections))
    try: bridge.update_scene(malformed)
    except ValueError: pass
    else: raise AssertionError('Malformed container accepted')
    assert counts == (len(bpy.data.objects), len(bpy.data.meshes), len(bpy.data.collections))

# Scene unit settings affect initial placement; later updates preserve user transforms.
bpy.context.scene.unit_settings.system = 'METRIC'
bpy.context.scene.unit_settings.scale_length = .01
centimetres = payload(20); centimetres['projectId'] = 'centimetre-scene'
bridge.update_scene(centimetres)
unit_obj = next(o for o in bpy.data.objects if o.get(bridge.PROJECT) == 'centimetre-scene')
bpy.context.view_layer.update()
assert abs(unit_obj.dimensions.x * bpy.context.scene.unit_settings.scale_length - .020) < 1e-8
unit_obj.scale = (.2,.2,.2)
bridge.update_scene(centimetres)
assert abs(unit_obj.scale.x-.2) < 1e-8
print('BLENDER_BRIDGE_BOUNDARIES_VERIFIED: malformed containers, centimetre scene, user scale')

# Object animation remains attached; mesh-data animation cannot be remapped safely.
obj.location = (1,2,3)
obj.keyframe_insert(data_path='location', frame=1)
obj.location = (7,8,9)
obj.keyframe_insert(data_path='location', frame=10)
action = obj.animation_data.action
bridge.update_scene(payload(13))
assert obj.animation_data.action == action
bpy.context.scene.frame_set(1)
assert obj.location[:] == (1.,2.,3.)
bpy.context.scene.frame_set(10)
assert obj.location[:] == (7.,8.,9.)
old_mesh = obj.data
old_mesh.animation_data_create()
counts = (len(bpy.data.objects), len(bpy.data.meshes))
try: bridge.update_scene(payload(14))
except ValueError as error: assert 'Animated mesh' in str(error)
else: raise AssertionError('Mesh animation discarded')
assert obj.data == old_mesh and obj.data.vertices[1].co.x == 13
assert counts == (len(bpy.data.objects), len(bpy.data.meshes))
print('BLENDER_BRIDGE_ANIMATION_VERIFIED: object keyframes preserved, mesh animation refused')

# Source material initializes a new object; Blender owns subsequent material edits.
colored = payload(); colored['projectId'] = 'source-material'
colored['bodies'][0]['material'] = {'name':'Copper','color':'#b87333','metallic':1.,'roughness':.2}
bridge.update_scene(colored)
colored_obj = next(o for o in bpy.data.objects if o.get(bridge.PROJECT) == 'source-material')
mat = colored_obj.data.materials[0]
shader = next(n for n in mat.node_tree.nodes if n.type == 'BSDF_PRINCIPLED')
assert abs(shader.inputs['Metallic'].default_value - 1.) < 1e-6
assert abs(shader.inputs['Roughness'].default_value - .2) < 1e-6
expected = ((184 / 255 + .055) / 1.055) ** 2.4
assert abs(shader.inputs['Base Color'].default_value[0] - expected) < 1e-6
shader.inputs['Roughness'].default_value = .75
colored['bodies'][0]['material']['roughness'] = .1
bridge.update_scene(colored)
assert colored_obj.data.materials[0] == mat
assert abs(shader.inputs['Roughness'].default_value - .75) < 1e-6
for invalid in [True, -1, 2, float('nan'), '0.5']:
    colored['bodies'][0]['material']['metallic'] = invalid
    counts = (len(bpy.data.meshes), len(bpy.data.materials))
    try: bridge.update_scene(colored)
    except ValueError: pass
    else: raise AssertionError('Invalid material accepted')
    assert counts == (len(bpy.data.meshes), len(bpy.data.materials))
print('BLENDER_SOURCE_MATERIAL_VERIFIED: linear color, metallic, roughness, user edit preservation, invalid input')

# Explicit source refresh replaces slots without mutating shared Blender materials.
colored['bodies'][0]['material']['metallic'] = .4
shared_mesh = bpy.data.meshes.new('External shared material mesh')
shared_mesh.materials.append(mat)
shared_obj = bpy.data.objects.new('External shared material object', shared_mesh)
bpy.context.scene.collection.objects.link(shared_obj)
bridge.register()
with tempfile.TemporaryDirectory() as directory:
    path = Path(directory)/'material.osv-blender.json'
    path.write_text(json.dumps(colored))
    assert bpy.ops.import_scene.solid_snapshot(filepath=str(path), update_materials=True) == {'FINISHED'}
    updated = colored_obj.data.materials[0]
    updated_shader = next(n for n in updated.node_tree.nodes if n.type == 'BSDF_PRINCIPLED')
    assert updated != mat and shared_obj.data.materials[0] == mat
    assert abs(shader.inputs['Roughness'].default_value - .75) < 1e-6
    assert abs(updated_shader.inputs['Metallic'].default_value - .4) < 1e-6
    assert abs(updated_shader.inputs['Roughness'].default_value - .1) < 1e-6
    del colored['bodies'][0]['material']
    path.write_text(json.dumps(colored))
    assert bpy.ops.import_scene.solid_snapshot(filepath=str(path), update_materials=True) == {'FINISHED'}
    assert len(colored_obj.data.materials) == 0
    assert shared_obj.data.materials[0] == mat
bridge.unregister()
print('BLENDER_MATERIAL_REFRESH_VERIFIED: operator option, source updates, removal, shared material isolation')
