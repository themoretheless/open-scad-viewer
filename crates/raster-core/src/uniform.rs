//! CPU-side mirror of the WGSL uniform structs. Field order and float offsets
//! are the GPU contract: `write_f32` emits exactly the layout the shaders
//! read, and the tests pin the offsets against the browser renderer.

/// Object uniform: model (16 floats) + nmat (16) + color (4) + style (4)
/// + morph (4) = 44 floats = 176 bytes.
///
/// `style` is (alpha, selected, edge, hovered); `morph.x` drives the GPU
/// vertex blend toward the slot-1 morph source positions.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ObjectUniform {
    /// Column-major 4x4 model matrix.
    pub model: [f32; 16],
    /// Column-major inverse-transpose of the model (normal matrix).
    pub nmat: [f32; 16],
    pub color: [f32; 4],
    pub style: [f32; 4],
    pub morph: [f32; 4],
}

pub const OBJECT_UNIFORM_FLOATS: usize = 44;
pub const OBJECT_UNIFORM_BYTES: u64 = 176;
pub const STYLE_FLOAT_OFFSET: usize = 36;
pub const STYLE_BYTE_OFFSET: u64 = 144;
pub const MORPH_FLOAT_OFFSET: usize = 40;
pub const MORPH_BYTE_OFFSET: u64 = 160;

impl ObjectUniform {
    pub const fn new(model: [f32; 16], nmat: [f32; 16], color: [f32; 4]) -> Self {
        Self { model, nmat, color, style: [1.0, 0.0, 0.0, 0.0], morph: [1.0, 0.0, 0.0, 0.0] }
    }

    /// Writes the full 44-float record into `out`.
    pub fn write_f32(&self, out: &mut [f32; OBJECT_UNIFORM_FLOATS]) {
        out[..16].copy_from_slice(&self.model);
        out[16..32].copy_from_slice(&self.nmat);
        out[32..36].copy_from_slice(&self.color);
        out[36..40].copy_from_slice(&self.style);
        out[40..44].copy_from_slice(&self.morph);
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
/// + section (4) + options (4) + inverse view-projection (16) = 52 floats =
/// 208 bytes. Shaders that do not read `inverseVP` still bind this buffer;
/// the member is simply unused there.
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
}

pub const SCENE_UNIFORM_FLOATS: usize = 52;
pub const SCENE_UNIFORM_BYTES: u64 = 208;

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
        }
    }

    /// Writes the full 52-float record into `out`.
    pub fn write_f32(&self, out: &mut [f32; SCENE_UNIFORM_FLOATS]) {
        out[..16].copy_from_slice(&self.view_projection);
        out[16..20].copy_from_slice(&self.eye);
        out[20..24].copy_from_slice(&self.light);
        out[24..28].copy_from_slice(&self.ambient);
        out[28..32].copy_from_slice(&self.section);
        out[32..36].copy_from_slice(&self.options);
        out[36..52].copy_from_slice(&self.inverse_vp);
    }
}
