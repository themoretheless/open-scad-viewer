struct Scene { vp: mat4x4f, eye: vec4f, light: vec4f, ambient: vec4f, section: vec4f, options: vec4f, inverseVP: mat4x4f, selectionColor: vec3f, hoverColor: vec3f, edgeColor: vec3f, xrayColor: vec3f, gridColor: vec3f, capColor: vec3f }
struct Obj { model: mat4x4f, nmat: mat4x4f, color: vec4f, style: vec4f, morph: vec4f, baseColor: vec3f, metallic: f32, emissive: vec3f, roughness: f32, materialId: f32 }
@group(0) @binding(0) var<uniform> sc: Scene;
@group(1) @binding(0) var<uniform> ob: Obj;
// Matcap capture, bound per renderer (not per object). The renderer always
// binds something: a 1x1 white dummy selects the procedural fallback below
// (textureDimensions == 1), a real capture switches to the textured path.
@group(2) @binding(0) var matcapTex: texture_2d<f32>;
@group(2) @binding(1) var matcapSampler: sampler;

struct V { @builtin(position) p: vec4f, @location(0) n: vec3f, @location(1) w: vec3f }

@vertex fn vs(@location(0) pos: vec3f, @location(1) norm: vec3f, @location(2) fromPos: vec3f) -> V {
  let local = mix(fromPos, pos, ob.morph.x);
  let wp = (ob.model * vec4f(local,1)).xyz;
  let wn = normalize((ob.nmat * vec4f(norm,0)).xyz);
  return V(sc.vp * vec4f(wp,1), wn, wp);
}

@fragment fn fs(v: V) -> @location(0) vec4f {
  if (sc.options.x > 0.5 && dot(v.w, sc.section.xyz) < sc.section.w) { discard; }
  // Cheap section-cap approximation (world-space epsilon highlight).
  if (sc.options.x > 0.5 && dot(v.w, sc.section.xyz) < sc.section.w + 0.02) { return vec4f(mix(ob.baseColor, sc.capColor, 0.6) + ob.emissive * 0.2, ob.style.x); }
  let N = normalize(v.n);
  let V2 = normalize(sc.eye.xyz - v.w);
  // Project the normal onto an orthonormal view basis: the 2D "material
  // capture" space, shared by the textured and procedural paths.
  let upRef = select(vec3f(0.0, 1.0, 0.0), vec3f(0.0, 0.0, 1.0), abs(V2.y) > 0.99);
  let right = normalize(cross(upRef, V2));
  let camUp = cross(V2, right);
  let m = vec2f(dot(N, right), dot(N, camUp));
  let selected = mix(ob.color.rgb, sc.selectionColor, ob.style.y * 0.48);
  let base = mix(selected, sc.hoverColor, ob.style.w * 0.38) * ob.baseColor;
  if (textureDimensions(matcapTex).x > 1u) {
    // Textured matcap: view-space normal projected into capture UV space,
    // tinted by the (selection/hover-aware) base color.
    let uv = m * 0.5 + vec2f(0.5, 0.5);
    let captured = textureSampleLevel(matcapTex, matcapSampler, uv, 0.0).rgb;
    return vec4f(captured * base + ob.emissive, ob.style.x);
  }
  // Soft top-left key light.
  let key = clamp(dot(m, vec2f(-0.35, 0.55)) * 0.5 + 0.55, 0.0, 1.0);
  // Fresnel rim, tinted slightly cool like a studio bounce.
  let rim = pow(1.0 - clamp(abs(dot(N, V2)), 0.0, 1.0), 2.5);
  // Specular blob: Gaussian around the key-light reflection spot.
  let blob = m - vec2f(-0.3, 0.45);
  let spec = exp(-dot(blob, blob) * 18.0) * mix(1.0, 0.4, clamp(ob.metallic, 0.0, 1.0));
  let c = base * (0.25 + 0.75 * key)
    + vec3f(0.9, 0.95, 1.0) * rim * 0.35
    + vec3f(spec) * 0.6 + ob.emissive;
  return vec4f(c, ob.style.x);
}
