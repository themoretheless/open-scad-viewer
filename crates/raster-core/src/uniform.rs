//! CPU-side mirror of the WGSL uniform structs. Field order and float offsets
//! are the GPU contract: `write_f32` emits exactly the layout the shaders
//! read, and the tests pin the offsets against the browser renderer.

/// Object uniform: model (16 floats) + nmat (16) + color (4) + style (4)
/// + morph (4) + baseColor (3) + metallic (1) + emissive (3) + roughness (1)
/// + materialId (1) + pad (3) = 56 floats = 224 bytes.
///
/// `style` is (alpha, selected, edge, hovered); `morph.x` drives the GPU
/// vertex blend toward the slot-1 morph source positions. The material tail
/// starts at float 44, so legacy field offsets are unchanged.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ObjectUniform {
    /// Column-major 4x4 model matrix.
    pub model: [f32; 16],
    /// Column-major inverse-transpose of the model (normal matrix).
    pub nmat: [f32; 16],
    pub color: [f32; 4],
    pub style: [f32; 4],
    pub morph: [f32; 4],
    /// Material base color multiplier (white = legacy look).
    pub base_color: [f32; 3],
    pub metallic: f32,
    /// Additive emission (black = none).
    pub emissive: [f32; 3],
    pub roughness: f32,
    /// Material preset/flags slot (0 = none).
    pub material_id: f32,
}

pub const OBJECT_UNIFORM_FLOATS: usize = 56;
pub const OBJECT_UNIFORM_BYTES: u64 = 224;
pub const STYLE_FLOAT_OFFSET: usize = 36;
pub const STYLE_BYTE_OFFSET: u64 = 144;
pub const MORPH_FLOAT_OFFSET: usize = 40;
pub const MORPH_BYTE_OFFSET: u64 = 160;
/// baseColor rgb at MATERIAL_FLOAT_OFFSET..+2, metallic at +3.
pub const MATERIAL_FLOAT_OFFSET: usize = 44;
pub const MATERIAL_BYTE_OFFSET: u64 = 176;
pub const METALLIC_FLOAT_OFFSET: usize = 47;
/// emissive rgb at EMISSIVE_FLOAT_OFFSET..+2, roughness at +3.
pub const EMISSIVE_FLOAT_OFFSET: usize = 48;
pub const EMISSIVE_BYTE_OFFSET: u64 = 192;
pub const ROUGHNESS_FLOAT_OFFSET: usize = 51;
pub const MATERIAL_ID_FLOAT_OFFSET: usize = 52;
pub const MATERIAL_ID_BYTE_OFFSET: u64 = 208;

impl ObjectUniform {
    pub const fn new(model: [f32; 16], nmat: [f32; 16], color: [f32; 4]) -> Self {
        Self {
            model, nmat, color,
            style: [1.0, 0.0, 0.0, 0.0], morph: [1.0, 0.0, 0.0, 0.0],
            base_color: [1.0, 1.0, 1.0], metallic: 0.0,
            emissive: [0.0, 0.0, 0.0], roughness: 0.7, material_id: 0.0,
        }
    }

    /// Writes the full 56-float record into `out`.
    pub fn write_f32(&self, out: &mut [f32; OBJECT_UNIFORM_FLOATS]) {
        out[..16].copy_from_slice(&self.model);
        out[16..32].copy_from_slice(&self.nmat);
        out[32..36].copy_from_slice(&self.color);
        out[36..40].copy_from_slice(&self.style);
        out[40..44].copy_from_slice(&self.morph);
        out[44..47].copy_from_slice(&self.base_color);
        out[47] = self.metallic;
        out[48..51].copy_from_slice(&self.emissive);
        out[51] = self.roughness;
        out[52] = self.material_id;
        out[53..56].fill(0.0);
    }
}

impl Default for ObjectUniform {
    fn default() -> Self {
        // Rest state: weight 1 renders the vertex-buffer target positions and
        // keeps the zero slot-1 morph dummy inert (mix(dummy, pos, 1) = pos),
        // matching the browser renderer's convention.
        Self::new(
            [1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0],
            [1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0],
            [0.8, 0.8, 0.8, 1.0],
        )
    }
}

/// Scene uniform: view-projection (16) + eye (4) + light (4) + ambient (4)
/// + section (4) + options (4) + inverse view-projection (16) + theme block
/// (selection/hover/edge/xray/grid/cap colors, 3 floats + pad each = 24) = 76
/// floats = 304 bytes. The theme tail starts at float 52, so legacy field
/// offsets are unchanged; shaders that do not read the theme simply bind a
/// smaller struct view of the same buffer.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SceneUniform {
    /// Column-major view-projection matrix.
    pub view_projection: [f32; 16],
    pub eye: [f32; 4],
    pub light: [f32; 4],
    pub ambient: [f32; 4],
    /// xyz: section plane normal, w: plane offset.
    pub section: [f32; 4],
    /// x: section enabled, y: grid step, z: grid extent, w: grid fade distance.
    pub options: [f32; 4],
    /// Column-major inverse view-projection (grid ray reconstruction).
    pub inverse_vp: [f32; 16],
    /// Selection tint (mesh family mixes toward it).
    pub selection_color: [f32; 3],
    /// Hover tint.
    pub hover_color: [f32; 3],
    /// Base wireframe edge color.
    pub edge_color: [f32; 3],
    /// X-ray (deep_mesh) fresnel glow color.
    pub xray_color: [f32; 3],
    /// Ground grid line color (axes stay fixed red/green).
    pub grid_color: [f32; 3],
    /// Section-cap fill color (mesh_section_cap and the epsilon accent).
    pub cap_color: [f32; 3],
}

pub const SCENE_UNIFORM_FLOATS: usize = 76;
pub const SCENE_UNIFORM_BYTES: u64 = 304;
/// Theme block: selectionColor at THEME_FLOAT_OFFSET..+2, then hover/edge/
/// xray/grid/cap colors every 4 floats (vec3 + pad, 16-byte aligned).
pub const THEME_FLOAT_OFFSET: usize = 52;
pub const THEME_BYTE_OFFSET: u64 = 208;
/// capColor rgb at CAP_FLOAT_OFFSET..+2.
pub const CAP_FLOAT_OFFSET: usize = 72;
pub const CAP_BYTE_OFFSET: u64 = 288;

impl SceneUniform {
    pub const fn new(view_projection: [f32; 16], eye: [f32; 4]) -> Self {
        Self {
            view_projection,
            eye,
            light: [0.55, 0.75, 0.45, 0.0],
            ambient: [0.22, 0.22, 0.24, 1.0],
            section: [0.0, 0.0, 1.0, 0.0],
            options: [0.0, 1.0, 0.0, 0.0],
            inverse_vp: [0.0; 16],
            // Defaults reproduce the previously hard-coded shader colors.
            selection_color: [1.0, 0.52, 0.06],
            hover_color: [0.12, 0.78, 1.0],
            edge_color: [0.025, 0.03, 0.04],
            xray_color: [1.0, 0.42, 0.06],
            grid_color: [0.42, 0.42, 0.42],
            cap_color: [0.85, 0.87, 0.9],
        }
    }

    /// Writes the full 76-float record into `out`.
    pub fn write_f32(&self, out: &mut [f32; SCENE_UNIFORM_FLOATS]) {
        out[..16].copy_from_slice(&self.view_projection);
        out[16..20].copy_from_slice(&self.eye);
        out[20..24].copy_from_slice(&self.light);
        out[24..28].copy_from_slice(&self.ambient);
        out[28..32].copy_from_slice(&self.section);
        out[32..36].copy_from_slice(&self.options);
        out[36..52].copy_from_slice(&self.inverse_vp);
        out[52..55].copy_from_slice(&self.selection_color);
        out[56..59].copy_from_slice(&self.hover_color);
        out[60..63].copy_from_slice(&self.edge_color);
        out[64..67].copy_from_slice(&self.xray_color);
        out[68..71].copy_from_slice(&self.grid_color);
        out[72..75].copy_from_slice(&self.cap_color);
        out[55] = 0.0;
        out[59] = 0.0;
        out[63] = 0.0;
        out[67] = 0.0;
        out[71] = 0.0;
        out[75] = 0.0;
    }
}
