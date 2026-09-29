"""Update a dedicated Blender collection from an openscad-viewer mesh snapshot."""
import math
import json
import re
from pathlib import Path
from bpy_extras.io_utils import ImportHelper
import bpy

bl_info = {'name': 'Solid Snapshot Bridge', 'author': 'OpenSCAD Viewer',
           'version': (0, 1, 0), 'blender': (4, 0, 0), 'category': 'Import-Export'}

SCHEMA = 'openscad-viewer/blender-1'
PROJECT = 'osv_project_id'
BODY = 'osv_body_id'


def validate(payload):
    if not isinstance(payload, dict):
        raise ValueError('Expected a snapshot object')
    if payload.get('schema') != SCHEMA or payload.get('units') != 'mm':
        raise ValueError('Expected an openscad-viewer Blender snapshot in millimetres')
    project = payload.get('projectId')
    if not isinstance(project, str) or not project.strip() or len(project) > 100:
        raise ValueError('Invalid project identity')
    bodies = payload.get('bodies')
    if not isinstance(bodies, list) or len(bodies) > 1200:
        raise ValueError('Invalid body list')
    ids = set()
    for body in bodies:
        if not isinstance(body, dict):
            raise ValueError('Expected a body object')
        identity = body.get('id')
        if not isinstance(identity, str) or not identity or identity in ids:
            raise ValueError('Body IDs must be unique nonempty strings')
        ids.add(identity)
        if not isinstance(body.get('name'), str) or len(body['name']) > 100:
            raise ValueError('Invalid body name')
        material = body.get('material')
        if material is not None:
            if (not isinstance(material, dict) or not isinstance(material.get('name'), str)
                    or not 1 <= len(material['name']) <= 100
                    or not isinstance(material.get('color'), str)
                    or not re.fullmatch(r'#[0-9a-fA-F]{6}', material['color'])):
                raise ValueError('Invalid source material')
            for key in ('metallic', 'roughness'):
                if key in material and (type(material[key]) not in (int, float)
                        or not math.isfinite(material[key]) or not 0 <= material[key] <= 1):
                    raise ValueError('Invalid source material parameter')
        mesh = body.get('mesh')
        if not isinstance(mesh, dict):
            raise ValueError('Expected a mesh object')
        positions, indices = mesh.get('positions'), mesh.get('indices')
        if not isinstance(positions, list) or not 9 <= len(positions) <= 150000 or len(positions) % 3:
            raise ValueError('Invalid positions')
        if any(type(x) not in (int, float) or not math.isfinite(x) or abs(x) > 1e6 for x in positions):
            raise ValueError('Invalid coordinates')
        if not isinstance(indices, list) or not 3 <= len(indices) <= 150000 or len(indices) % 3:
            raise ValueError('Invalid triangles')
        if any(type(i) is not int or i < 0 or i >= len(positions)//3 for i in indices):
            raise ValueError('Invalid vertex index')
    return project, bodies


def create_source_material(source):
    material = bpy.data.materials.new(source['name'])
    material.use_nodes = True
    shader = next(n for n in material.node_tree.nodes if n.type == 'BSDF_PRINCIPLED')
    srgb = [int(source['color'][i:i+2], 16) / 255 for i in (1, 3, 5)]
    linear = [c / 12.92 if c <= .04045 else ((c + .055) / 1.055) ** 2.4 for c in srgb]
    color = (*linear, 1.)
    shader.inputs['Base Color'].default_value = color
    shader.inputs['Metallic'].default_value = source.get('metallic', 0.)
    shader.inputs['Roughness'].default_value = source.get('roughness', .5)
    material.diffuse_color = color
    material.metallic = source.get('metallic', 0.)
    material.roughness = source.get('roughness', .5)
    return material


def update_scene(payload, update_materials=False):
    project, bodies = validate(payload)
    if bpy.context.mode != 'OBJECT':
        raise ValueError('Switch Blender to Object Mode before updating')
    collections = [c for c in bpy.data.collections if c.get(PROJECT) == project]
    if len(collections) > 1:
        raise ValueError('Duplicate bridge project collections')
    if collections and (collections[0].library or collections[0].override_library):
        raise ValueError('Bridge collection must be local and editable')
    unit_scale = bpy.context.scene.unit_settings.scale_length
    if not math.isfinite(unit_scale) or unit_scale <= 0:
        raise ValueError('Scene unit scale must be positive and finite')
    existing = {}
    for obj in bpy.data.objects:
        if obj.get(PROJECT) != project:
            continue
        identity = obj.get(BODY)
        if not isinstance(identity, str) or not identity:
            raise ValueError('Bridge object is missing its body identity')
        if identity in existing:
            raise ValueError('Duplicate bridge body identity in Blender')
        if obj.type != 'MESH' or obj.library or obj.override_library or obj.data.library or obj.data.override_library:
            raise ValueError('Bridge objects must be local meshes')
        if obj.vertex_groups or obj.data.shape_keys:
            raise ValueError('Vertex groups and shape keys require an independent copy before updating')
        mesh = obj.data
        if mesh.animation_data:
            raise ValueError('Animated mesh data requires an independent copy before updating')
        if mesh.uv_layers or mesh.color_attributes or mesh.get('user_metadata'):
            raise ValueError('UVs and mesh metadata require an independent copy before updating')
        allowed = {'position', '.edge_verts', '.corner_vert', '.corner_edge',
                   '.select_vert', '.select_edge', '.select_poly', 'sharp_face'}
        if mesh.has_custom_normals or any(p.use_smooth for p in mesh.polygons):
            raise ValueError('Custom normals and smooth shading require an independent copy before updating')
        if any(a.name not in allowed for a in mesh.attributes) or len(mesh.materials) > 1 or mesh.keys():
            raise ValueError('Mesh attributes and multiple material slots require an independent copy before updating')
        existing[identity] = obj
    # Construct and validate all replacement meshes before touching existing objects.
    staged = []
    staged_materials = []
    try:
        for body in bodies:
            mesh = bpy.data.meshes.new(body['name'] + ' · Solid')
            staged.append((body, mesh))
            p, ix = body['mesh']['positions'], body['mesh']['indices']
            mesh.from_pydata([p[i:i+3] for i in range(0, len(p), 3)], [],
                             [ix[i:i+3] for i in range(0, len(ix), 3)])
            if mesh.validate():
                raise ValueError('Blender repaired invalid mesh data; update refused')
            mesh.update()
            if (body['id'] not in existing or update_materials) and body.get('material') is not None:
                material = create_source_material(body['material'])
                staged_materials.append(material)
                mesh.materials.append(material)
    except Exception:
        for _, mesh in staged:
            bpy.data.meshes.remove(mesh)
        for material in staged_materials:
            bpy.data.materials.remove(material)
        raise
    collection = collections[0] if collections else bpy.data.collections.new('Solid · ' + project)
    if not collections:
        collection[PROJECT] = project
        bpy.context.scene.collection.children.link(collection)
    imported = set()
    for body, mesh in staged:
        obj = existing.get(body['id'])
        if obj is None:
            obj = bpy.data.objects.new(body['name'], mesh)
            obj[PROJECT], obj[BODY] = project, body['id']
            obj.scale = (0.001 / unit_scale,) * 3
            collection.objects.link(obj)
        else:
            old = obj.data
            # Preserve Blender materials unless source refresh was explicitly selected.
            if not update_materials:
                for material in old.materials:
                    mesh.materials.append(material)
            obj.data = mesh
            if old.users == 0:
                bpy.data.meshes.remove(old)
        obj['osv_source_name'] = body['name']
        obj['osv_missing_from_source'] = False
        imported.add(body['id'])
    # Missing bodies are marked, not destroyed: their animation/modifiers may be valuable.
    for identity, obj in existing.items():
        if identity not in imported:
            obj['osv_missing_from_source'] = True
    return {'updated': len(staged), 'missing': len(set(existing)-imported)}


class SOLID_OT_import_snapshot(bpy.types.Operator, ImportHelper):
    bl_idname = 'import_scene.solid_snapshot'
    bl_label = 'Update Solid Snapshot'
    bl_options = {'REGISTER', 'UNDO'}
    update_materials: bpy.props.BoolProperty(
        name='Update materials from Solid', default=False,
        description='Replace material slots on imported objects; clear them when the source has no material. Shared Blender materials remain unchanged')
    filename_ext = '.osv-blender.json'
    filter_glob: bpy.props.StringProperty(default='*.osv-blender.json', options={'HIDDEN'})

    def execute(self, context):
        try:
            path = Path(self.filepath)
            if path.stat().st_size > 64_000_000:
                raise ValueError('Snapshot exceeds 64 MB')
            result = update_scene(json.loads(path.read_text(encoding='utf-8')), self.update_materials)
        except (ValueError, OSError, TypeError, AttributeError) as error:
            self.report({'ERROR'}, str(error))
            return {'CANCELLED'}
        self.report({'INFO'}, f"Updated {result['updated']} bodies; {result['missing']} missing")
        return {'FINISHED'}


def import_menu(self, context):
    self.layout.operator(SOLID_OT_import_snapshot.bl_idname, text='Solid Snapshot (.osv-blender.json)')


def register():
    bpy.utils.register_class(SOLID_OT_import_snapshot)
    bpy.types.TOPBAR_MT_file_import.append(import_menu)


def unregister():
    bpy.types.TOPBAR_MT_file_import.remove(import_menu)
    bpy.utils.unregister_class(SOLID_OT_import_snapshot)


if __name__ == '__main__':
    register()
