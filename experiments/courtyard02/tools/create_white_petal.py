"""Create the editable petal, portable GLB, and the same mesh for Belfast.

Run with: blender --background --factory-startup --python tools/create_white_petal.py
"""
import json
import math
from pathlib import Path

import bpy
from mathutils import Vector


ROOT = Path(__file__).resolve().parents[1]
ASSETS = ROOT / "assets"
PUBLIC = ROOT / "public"
ASSETS.mkdir(exist_ok=True)
PUBLIC.mkdir(exist_ok=True)
bpy.ops.object.select_all(action="SELECT")
bpy.ops.object.delete(use_global=False)

# Local coordinates: base at the origin, tip at +Y, front at +Z.
# Two interior rows keep a cupped silhouette for petals around 0.02 scene units long.
rows, columns = 3, 2
points, uvs = [(0.0, 0.0, 0.0)], [(0.5, 0.0)]
for row in range(1, rows):
    t = (1.0 - math.cos(math.pi * row / rows)) * 0.5
    envelope = math.sin(math.pi * t)
    width = 0.42 * envelope**0.55 * (0.5 + 0.75 * t)
    for column in range(columns + 1):
        u = column * 2.0 / columns - 1.0
        x = width * u + 0.025 * t * t
        z = 0.08 * t * t - 0.04 * envelope
        z += 0.11 * u * u * envelope**0.7 + 0.015 * u * envelope
        z -= 0.018 * math.exp(-u * u / 0.06) * envelope
        z += 0.006 * math.sin(t * 4.0 * math.pi + u) * u**4 * envelope
        points.append((x, t, z))
        uvs.append(((u + 1.0) * 0.5, t))
points.append((0.025, 1.0, 0.08))
uvs.append((0.5, 1.0))

faces = [(0, 2 + i, 1 + i) for i in range(columns)]
for row in range(rows - 2):
    for column in range(columns):
        a = 1 + row * (columns + 1) + column
        b = a + columns + 1
        faces.extend([(a, a + 1, b), (a + 1, b + 1, b)])
last_row = 1 + (rows - 2) * (columns + 1)
tip = len(points) - 1
faces.extend((last_row + i, last_row + i + 1, tip) for i in range(columns))

mesh = bpy.data.meshes.new("WhitePetalMesh")
# Convert the exported Y-up coordinates to Blender's Z-up coordinates.
mesh.from_pydata([(x, -z, y) for x, y, z in points], [], faces)
mesh.update()
petal = bpy.data.objects.new("WhitePetal", mesh)
bpy.context.collection.objects.link(petal)
bpy.context.view_layer.objects.active = petal
petal.select_set(True)
for polygon in mesh.polygons:
    polygon.use_smooth = True
uv_layer = mesh.uv_layers.new(name="UVMap")
for loop in mesh.loops:
    uv_layer.data[loop.index].uv = uvs[loop.vertex_index]

colors = mesh.color_attributes.new(name="PetalTint", type="FLOAT_COLOR", domain="POINT")


def srgb_to_linear(value):
    return value / 12.92 if value <= 0.04045 else ((value + 0.055) / 1.055) ** 2.4


# Brightened from the bright-half median of 215 likely flower reference splats.
cream = tuple(srgb_to_linear(c / 255.0) for c in (238, 237, 224))
base_green = tuple(srgb_to_linear(c / 255.0) for c in (166, 181, 138))
for index, (_, t) in enumerate(uvs):
    blend = min(1.0, t / 0.5)
    blend = blend * blend * (3.0 - 2.0 * blend)
    colors.data[index].color = tuple(
        base + (tip - base) * blend for base, tip in zip(base_green, cream)
    ) + (1.0,)
material = bpy.data.materials.new("IvoryWhitePetal")
material.use_nodes = True
material.use_backface_culling = False
shader = material.node_tree.nodes.get("Principled BSDF")
shader.inputs["Roughness"].default_value = 0.72
color_node = material.node_tree.nodes.new("ShaderNodeVertexColor")
color_node.layer_name = "PetalTint"
material.node_tree.links.new(color_node.outputs["Color"], shader.inputs["Base Color"])
mesh.materials.append(material)

vertices = []
for vertex in mesh.vertices:
    p, n = vertex.co, vertex.normal
    values = [p.x, p.z, -p.y, n.x, n.z, -n.y, *uvs[vertex.index], *colors.data[vertex.index].color]
    vertices.append([round(float(value), 7) for value in values])
asset = {
    "version": 1,
    "name": "WhitePetal",
    "coordinates": "y-up, base origin, tip +Y, front +Z",
    "vertex_layout": "position.xyz, normal.xyz, uv.xy, linear_color.rgba",
    "vertices": vertices,
    "indices": [index for triangle in faces for index in triangle],
}
(PUBLIC / "white-petal.mesh.json").write_text(json.dumps(asset, separators=(",", ":")) + "\n")
bpy.ops.export_scene.gltf(
    filepath=str(PUBLIC / "white-petal.glb"),
    export_format="GLB",
    use_selection=True,
    export_yup=True,
    export_animations=False,
    export_vertex_color="NAME",
    export_vertex_color_name="PetalTint",
    export_all_vertex_colors=False,
)

# A small studio scene accompanies the editable model and renders its asset preview.
scene = bpy.context.scene
scene.render.engine = "CYCLES"
scene.cycles.device = "CPU"
scene.cycles.samples = 32
scene.render.resolution_x = 640
scene.render.resolution_y = 720
scene.render.resolution_percentage = 100
scene.render.image_settings.file_format = "PNG"
scene.render.film_transparent = False
scene.world.use_nodes = True
scene.world.node_tree.nodes["Background"].inputs[0].default_value = (0.045, 0.055, 0.065, 1.0)
scene.world.node_tree.nodes["Background"].inputs[1].default_value = 0.7
scene.view_settings.view_transform = "Standard"


def aim(obj, target):
    obj.rotation_euler = (Vector(target) - obj.location).to_track_quat("-Z", "Y").to_euler()


bpy.ops.object.camera_add(location=(0.85, -2.8, 1.45))
camera = bpy.context.object
camera.name = "PetalPreviewCamera"
camera.data.type = "ORTHO"
camera.data.ortho_scale = 1.35
aim(camera, (0.0, 0.0, 0.52))
scene.camera = camera
for name, location, energy, size in [
    ("SoftKey", (-1.6, -2.0, 2.6), 110, 2.2),
    ("SoftFill", (1.8, -0.5, 1.2), 35, 1.8),
]:
    bpy.ops.object.light_add(type="AREA", location=location)
    light = bpy.context.object
    light.name = name
    light.data.energy = energy
    light.data.shape = "DISK"
    light.data.size = size
    aim(light, (0.0, 0.0, 0.5))
bpy.ops.object.select_all(action="DESELECT")
petal.select_set(True)
bpy.context.view_layer.objects.active = petal
for screen in bpy.data.screens:
    for area in screen.areas:
        if area.type == "VIEW_3D":
            area.spaces.active.region_3d.view_perspective = "CAMERA"
scene.render.filepath = str(ASSETS / "white-petal-preview.png")
bpy.context.preferences.filepaths.save_version = 0
bpy.ops.wm.save_as_mainfile(filepath=str(ASSETS / "white-petal.blend"))
print(f"Created WhitePetal: {len(vertices)} vertices, {len(faces)} triangles")
bpy.ops.render.render(write_still=True)
