var e={floats:96,bytes:384,vpFloatOffset:0,vpByteOffset:0,eyeFloatOffset:16,eyeByteOffset:64,lightFloatOffset:20,lightByteOffset:80,ambientFloatOffset:24,ambientByteOffset:96,sectionFloatOffset:28,sectionByteOffset:112,optionsFloatOffset:32,optionsByteOffset:128,inverseVPFloatOffset:36,inverseVPByteOffset:144,themeFloatOffset:52,themeByteOffset:208,hoverFloatOffset:56,edgeFloatOffset:60,xrayFloatOffset:64,gridFloatOffset:68,capFloatOffset:72,capByteOffset:288,lightVPFloatOffset:76,lightVPByteOffset:304,shadowFloatOffset:92,shadowByteOffset:368},t={floats:56,bytes:224,styleFloatOffset:36,styleByteOffset:144,morphFloatOffset:40,morphByteOffset:160,materialFloatOffset:44,materialByteOffset:176,metallicFloatOffset:47,emissiveFloatOffset:48,emissiveByteOffset:192,roughnessFloatOffset:51,materialIdFloatOffset:52,materialIdByteOffset:208};function n(e){if(!e.includes(`var<uniform> ob: Obj;`))throw Error(`Immediate shader contract changed: missing object uniform`);return`requires immediate_address_space;
var<immediate> im_style: vec4f;
fn objectStyle() -> vec4f { return im_style; }
`+e.replaceAll(`ob.style`,`objectStyle()`)}function r(){return typeof navigator<`u`&&!!navigator.gpu&&!0===navigator.gpu.wgslLanguageFeatures?.has(`immediate_address_space`)}function i(e,t){let n=[[`var<uniform> ob: Obj;`,`var<storage, read> objects: array<Obj>;`],[`struct ${t} {`,`struct ${t} { @location(2) @interpolate(flat) instance: u32,`],[`@vertex fn vs(`,`@vertex fn vs(@builtin(instance_index) instance: u32, `],[` -> ${t} {`,` -> ${t} {\n  let ob = objects[instance];`],[`return ${t}(`,`return ${t}(instance, `],[`@fragment fn fs(v: ${t}) -> @location(0) vec4f {`,`@fragment fn fs(v: ${t}) -> @location(0) vec4f {\n  let ob = objects[v.instance];`]];for(let[t,r]of n){if(e.split(t).length!==2)throw Error(`Instanced shader contract changed: ${t}`);e=e.replace(t,r)}return e}var a=new Map;function o(e){if(a.has(e.id))throw Error(`Shader registry: duplicate id '${e.id}'`);return a.set(e.id,e),e}function s(e){let t=a.get(e);if(!t)throw Error(`Shader registry: unknown shader '${e}'`);return t}o({id:`mesh`,source:`struct Scene { vp: mat4x4f, eye: vec4f, light: vec4f, ambient: vec4f, section: vec4f, options: vec4f, inverseVP: mat4x4f, selectionColor: vec3f, hoverColor: vec3f, edgeColor: vec3f, xrayColor: vec3f, gridColor: vec3f, capColor: vec3f, lightVP: mat4x4f, shadowParams: vec4f }
struct Obj { model: mat4x4f, nmat: mat4x4f, color: vec4f, style: vec4f, morph: vec4f, baseColor: vec3f, metallic: f32, emissive: vec3f, roughness: f32, materialId: f32 }
@group(0) @binding(0) var<uniform> sc: Scene;
@group(1) @binding(0) var<uniform> ob: Obj;
@group(2) @binding(0) var shadowMap: texture_depth_2d;
@group(2) @binding(1) var shadowSampler: sampler_comparison;
struct V { @builtin(position) p: vec4f, @location(0) n: vec3f, @location(1) w: vec3f }
fn shadowFactor(wp: vec3f) -> f32 {
if (sc.shadowParams.x < 0.5) { return 1.0; }
let lp = sc.lightVP * vec4f(wp, 1.0);
let ndc = lp.xyz / lp.w;
let uv = ndc.xy * vec2f(0.5, -0.5) + vec2f(0.5);
let inb = ndc.z >= 0.0 && ndc.z <= 1.0 && uv.x >= 0.0 && uv.x <= 1.0 && uv.y >= 0.0 && uv.y <= 1.0;
let texel = sc.shadowParams.y;
let depth = ndc.z - sc.shadowParams.z;
var sum = 0.0;
for (var dy = -1; dy <= 1; dy++) {
for (var dx = -1; dx <= 1; dx++) {
sum += textureSampleCompare(shadowMap, shadowSampler, clamp(uv, vec2f(0.0), vec2f(1.0)) + vec2f(f32(dx) * texel, f32(dy) * texel), depth);
}
}
return select(1.0, mix(1.0, sum / 9.0, sc.shadowParams.w), inb);
}
@vertex fn vs(@location(0) pos: vec3f, @location(1) norm: vec3f, @location(2) fromPos: vec3f) -> V {
let local = mix(fromPos, pos, ob.morph.x);
let wp = (ob.model * vec4f(local,1)).xyz;
let wn = normalize((ob.nmat * vec4f(norm,0)).xyz);
return V(sc.vp * vec4f(wp,1), wn, wp);
}
@fragment fn fs(v: V) -> @location(0) vec4f {
if (sc.options.x > 0.5 && dot(v.w, sc.section.xyz) < sc.section.w) { discard; }
let shadow = shadowFactor(v.w);
if (sc.options.x > 0.5 && dot(v.w, sc.section.xyz) < sc.section.w + 0.02) { return vec4f(mix(ob.baseColor, sc.capColor, 0.6) + ob.emissive * 0.2, ob.style.x); }
let N = normalize(v.n);
let L = normalize(sc.light.xyz);
let V2 = normalize(sc.eye.xyz - v.w);
let H = normalize(L + V2);
let d = max(dot(N, L), 0.0);
let s = pow(max(dot(N, H), 0.0), 40.0);
let bd = max(dot(-N, L), 0.0) * 0.25;
let selected = mix(ob.color.rgb, sc.selectionColor, ob.style.y * 0.48);
let base = mix(selected, sc.hoverColor, ob.style.w * 0.38) * ob.baseColor;
let c = sc.ambient.rgb * base * shadow + d * base + s * vec3f(0.25) * (1.0 - ob.metallic) + bd * base * 0.5 + ob.emissive;
return vec4f(c, ob.style.x);
}`,kind:`object`,blend:`none`,depth:{writeEnabled:!0,compare:`less`},topology:`triangle-list`,cullMode:`none`,vertexLayout:`mesh`,supportsVariants:!0,usesShadowBinding:!0});for(let[e,t,n,r,i]of[[`meshPbr`,`struct Scene { vp: mat4x4f, eye: vec4f, light: vec4f, ambient: vec4f, section: vec4f, options: vec4f, inverseVP: mat4x4f, selectionColor: vec3f, hoverColor: vec3f, edgeColor: vec3f, xrayColor: vec3f, gridColor: vec3f, capColor: vec3f, lightVP: mat4x4f, shadowParams: vec4f }
struct Obj { model: mat4x4f, nmat: mat4x4f, color: vec4f, style: vec4f, morph: vec4f, baseColor: vec3f, metallic: f32, emissive: vec3f, roughness: f32, materialId: f32 }
@group(0) @binding(0) var<uniform> sc: Scene;
@group(1) @binding(0) var<uniform> ob: Obj;
@group(2) @binding(0) var envTex: texture_2d<f32>;
@group(2) @binding(1) var envSampler: sampler;
@group(2) @binding(2) var shadowMap: texture_depth_2d;
@group(2) @binding(3) var shadowSampler: sampler_comparison;
struct V { @builtin(position) p: vec4f, @location(0) n: vec3f, @location(1) w: vec3f }
fn shadowFactor(wp: vec3f) -> f32 {
if (sc.shadowParams.x < 0.5) { return 1.0; }
let lp = sc.lightVP * vec4f(wp, 1.0);
let ndc = lp.xyz / lp.w;
let uv = ndc.xy * vec2f(0.5, -0.5) + vec2f(0.5);
let inb = ndc.z >= 0.0 && ndc.z <= 1.0 && uv.x >= 0.0 && uv.x <= 1.0 && uv.y >= 0.0 && uv.y <= 1.0;
let texel = sc.shadowParams.y;
let depth = ndc.z - sc.shadowParams.z;
var sum = 0.0;
for (var dy = -1; dy <= 1; dy++) {
for (var dx = -1; dx <= 1; dx++) {
sum += textureSampleCompare(shadowMap, shadowSampler, clamp(uv, vec2f(0.0), vec2f(1.0)) + vec2f(f32(dx) * texel, f32(dy) * texel), depth);
}
}
return select(1.0, mix(1.0, sum / 9.0, sc.shadowParams.w), inb);
}
@vertex fn vs(@location(0) pos: vec3f, @location(1) norm: vec3f, @location(2) fromPos: vec3f) -> V {
let local = mix(fromPos, pos, ob.morph.x);
let wp = (ob.model * vec4f(local,1)).xyz;
let wn = normalize((ob.nmat * vec4f(norm,0)).xyz);
return V(sc.vp * vec4f(wp,1), wn, wp);
}
fn fresnelSchlick(cosTheta: f32, F0: vec3f) -> vec3f {
return F0 + (vec3f(1.0) - F0) * pow(clamp(1.0 - cosTheta, 0.0, 1.0), 5.0);
}
fn distributionGGX(N: vec3f, H: vec3f, rough: f32) -> f32 {
let a = max(rough * rough, 0.001);
let a2 = a * a;
let ndh = max(dot(N, H), 0.0);
let denom = ndh * ndh * (a2 - 1.0) + 1.0;
return a2 / (3.14159265 * denom * denom);
}
fn geometrySchlickGGX(ndv: f32, rough: f32) -> f32 {
let r = rough + 1.0;
let k = (r * r) / 8.0;
return ndv / (ndv * (1.0 - k) + k);
}
fn geometrySmith(N: vec3f, V2: vec3f, L: vec3f, rough: f32) -> f32 {
return geometrySchlickGGX(max(dot(N, V2), 0.0), rough)
* geometrySchlickGGX(max(dot(N, L), 0.0), rough);
}
fn envUv(dir: vec3f) -> vec2f {
let d = normalize(dir);
let azimuth = select(atan2(d.y, d.x), 0.0, abs(d.x) + abs(d.y) < 1e-6);
let u = azimuth / 6.2831853 + 0.5;
let v = acos(clamp(d.z, -1.0, 1.0)) / 3.14159265;
return vec2f(u, v);
}
fn envSample(dir: vec3f, lod: f32) -> vec3f {
return textureSampleLevel(envTex, envSampler, envUv(dir), lod).rgb;
}
fn envIrradiance(N: vec3f) -> vec3f {
let upRef = select(vec3f(0.0, 1.0, 0.0), vec3f(0.0, 0.0, 1.0), abs(N.y) > 0.99);
let t = normalize(cross(upRef, N));
let b = cross(N, t);
let w = 0.55;
return (envSample(N, 0.0)
+ envSample(normalize(N + t * w), 0.0)
+ envSample(normalize(N - t * w), 0.0)
+ envSample(normalize(N + b * w), 0.0)
+ envSample(normalize(N - b * w), 0.0)) * 0.2;
}
@fragment fn fs(v: V) -> @location(0) vec4f {
if (sc.options.x > 0.5 && dot(v.w, sc.section.xyz) < sc.section.w) { discard; }
let shadow = shadowFactor(v.w);
if (sc.options.x > 0.5 && dot(v.w, sc.section.xyz) < sc.section.w + 0.02) { return vec4f(mix(ob.baseColor, sc.capColor, 0.6) + ob.emissive * 0.2, ob.style.x); }
let N = normalize(v.n);
let L = normalize(sc.light.xyz);
let V2 = normalize(sc.eye.xyz - v.w);
let H = normalize(L + V2);
let selected = mix(ob.color.rgb, sc.selectionColor, ob.style.y * 0.48);
let albedo = mix(selected, sc.hoverColor, ob.style.w * 0.38) * ob.baseColor;
let metallic = clamp(ob.metallic, 0.0, 1.0);
let rough = clamp(ob.roughness, 0.05, 1.0);
let F0 = mix(vec3f(0.04), albedo, metallic);
let ndf = distributionGGX(N, H, rough);
let g = geometrySmith(N, V2, L, rough);
let f = fresnelSchlick(max(dot(H, V2), 0.0), F0);
let specular = (ndf * g * f) / max(4.0 * max(dot(N, V2), 0.0) * max(dot(N, L), 0.0), 0.0001);
let kd = (vec3f(1.0) - f) * (1.0 - metallic);
let ndl = max(dot(N, L), 0.0);
let bd = max(dot(-N, L), 0.0) * 0.25;
var c: vec3f;
if (textureDimensions(envTex).x > 1u) {
let irradiance = envIrradiance(N);
var R = reflect(-V2, N);
R = normalize(mix(R, N, rough * rough));
var envSpec: vec3f;
if (textureNumLevels(envTex) > 1u) {
envSpec = envSample(R, rough * f32(textureNumLevels(envTex) - 1u));
} else {
envSpec = envSample(R, 0.0) * pow(1.0 - rough, 2.0);
}
let fEnv = fresnelSchlick(max(dot(N, V2), 0.0), F0);
c = irradiance * albedo * (vec3f(1.0) - fEnv) * (1.0 - metallic) * shadow
+ envSpec * fEnv
+ sc.ambient.rgb * albedo * 0.25 * shadow + ob.emissive;
} else {
c = (kd * albedo / 3.14159265 + specular) * ndl
+ sc.ambient.rgb * albedo * shadow + bd * albedo * 0.5 + ob.emissive;
}
c = c / (c + vec3f(1.0));
c = pow(c, vec3f(1.0 / 2.2));
return vec4f(c, ob.style.x);
}`,!1,!0,!1],[`meshMatcap`,`struct Scene { vp: mat4x4f, eye: vec4f, light: vec4f, ambient: vec4f, section: vec4f, options: vec4f, inverseVP: mat4x4f, selectionColor: vec3f, hoverColor: vec3f, edgeColor: vec3f, xrayColor: vec3f, gridColor: vec3f, capColor: vec3f, lightVP: mat4x4f, shadowParams: vec4f }
struct Obj { model: mat4x4f, nmat: mat4x4f, color: vec4f, style: vec4f, morph: vec4f, baseColor: vec3f, metallic: f32, emissive: vec3f, roughness: f32, materialId: f32 }
@group(0) @binding(0) var<uniform> sc: Scene;
@group(1) @binding(0) var<uniform> ob: Obj;
@group(2) @binding(0) var matcapTex: texture_2d<f32>;
@group(2) @binding(1) var matcapSampler: sampler;
@group(2) @binding(2) var shadowMap: texture_depth_2d;
@group(2) @binding(3) var shadowSampler: sampler_comparison;
struct V { @builtin(position) p: vec4f, @location(0) n: vec3f, @location(1) w: vec3f }
fn shadowFactor(wp: vec3f) -> f32 {
if (sc.shadowParams.x < 0.5) { return 1.0; }
let lp = sc.lightVP * vec4f(wp, 1.0);
let ndc = lp.xyz / lp.w;
let uv = ndc.xy * vec2f(0.5, -0.5) + vec2f(0.5);
let inb = ndc.z >= 0.0 && ndc.z <= 1.0 && uv.x >= 0.0 && uv.x <= 1.0 && uv.y >= 0.0 && uv.y <= 1.0;
let texel = sc.shadowParams.y;
let depth = ndc.z - sc.shadowParams.z;
var sum = 0.0;
for (var dy = -1; dy <= 1; dy++) {
for (var dx = -1; dx <= 1; dx++) {
sum += textureSampleCompare(shadowMap, shadowSampler, clamp(uv, vec2f(0.0), vec2f(1.0)) + vec2f(f32(dx) * texel, f32(dy) * texel), depth);
}
}
return select(1.0, mix(1.0, sum / 9.0, sc.shadowParams.w), inb);
}
@vertex fn vs(@location(0) pos: vec3f, @location(1) norm: vec3f, @location(2) fromPos: vec3f) -> V {
let local = mix(fromPos, pos, ob.morph.x);
let wp = (ob.model * vec4f(local,1)).xyz;
let wn = normalize((ob.nmat * vec4f(norm,0)).xyz);
return V(sc.vp * vec4f(wp,1), wn, wp);
}
@fragment fn fs(v: V) -> @location(0) vec4f {
if (sc.options.x > 0.5 && dot(v.w, sc.section.xyz) < sc.section.w) { discard; }
let shadow = shadowFactor(v.w);
if (sc.options.x > 0.5 && dot(v.w, sc.section.xyz) < sc.section.w + 0.02) { return vec4f(mix(ob.baseColor, sc.capColor, 0.6) + ob.emissive * 0.2, ob.style.x); }
let N = normalize(v.n);
let V2 = normalize(sc.eye.xyz - v.w);
let upRef = select(vec3f(0.0, 1.0, 0.0), vec3f(0.0, 0.0, 1.0), abs(V2.y) > 0.99);
let right = normalize(cross(upRef, V2));
let camUp = cross(V2, right);
let m = vec2f(dot(N, right), dot(N, camUp));
let selected = mix(ob.color.rgb, sc.selectionColor, ob.style.y * 0.48);
let base = mix(selected, sc.hoverColor, ob.style.w * 0.38) * ob.baseColor;
if (textureDimensions(matcapTex).x > 1u) {
let uv = m * 0.5 + vec2f(0.5, 0.5);
let captured = textureSampleLevel(matcapTex, matcapSampler, uv, 0.0).rgb;
return vec4f(captured * base + ob.emissive, ob.style.x);
}
let key = clamp(dot(m, vec2f(-0.35, 0.55)) * 0.5 + 0.55, 0.0, 1.0);
let rim = pow(1.0 - clamp(abs(dot(N, V2)), 0.0, 1.0), 2.5);
let blob = m - vec2f(-0.3, 0.45);
let spec = exp(-dot(blob, blob) * 18.0) * mix(1.0, 0.4, clamp(ob.metallic, 0.0, 1.0));
let c = base * (0.25 * shadow + 0.75 * key)
+ vec3f(0.9, 0.95, 1.0) * rim * 0.35
+ vec3f(spec) * 0.6 + ob.emissive;
return vec4f(c, ob.style.x);
}`,!0,!1,!1],[`meshToon`,`struct Scene { vp: mat4x4f, eye: vec4f, light: vec4f, ambient: vec4f, section: vec4f, options: vec4f, inverseVP: mat4x4f, selectionColor: vec3f, hoverColor: vec3f, edgeColor: vec3f, xrayColor: vec3f, gridColor: vec3f, capColor: vec3f, lightVP: mat4x4f, shadowParams: vec4f }
struct Obj { model: mat4x4f, nmat: mat4x4f, color: vec4f, style: vec4f, morph: vec4f, baseColor: vec3f, metallic: f32, emissive: vec3f, roughness: f32, materialId: f32 }
@group(0) @binding(0) var<uniform> sc: Scene;
@group(1) @binding(0) var<uniform> ob: Obj;
@group(2) @binding(0) var shadowMap: texture_depth_2d;
@group(2) @binding(1) var shadowSampler: sampler_comparison;
struct V { @builtin(position) p: vec4f, @location(0) n: vec3f, @location(1) w: vec3f }
fn shadowFactor(wp: vec3f) -> f32 {
if (sc.shadowParams.x < 0.5) { return 1.0; }
let lp = sc.lightVP * vec4f(wp, 1.0);
let ndc = lp.xyz / lp.w;
let uv = ndc.xy * vec2f(0.5, -0.5) + vec2f(0.5);
let inb = ndc.z >= 0.0 && ndc.z <= 1.0 && uv.x >= 0.0 && uv.x <= 1.0 && uv.y >= 0.0 && uv.y <= 1.0;
let texel = sc.shadowParams.y;
let depth = ndc.z - sc.shadowParams.z;
var sum = 0.0;
for (var dy = -1; dy <= 1; dy++) {
for (var dx = -1; dx <= 1; dx++) {
sum += textureSampleCompare(shadowMap, shadowSampler, clamp(uv, vec2f(0.0), vec2f(1.0)) + vec2f(f32(dx) * texel, f32(dy) * texel), depth);
}
}
return select(1.0, mix(1.0, sum / 9.0, sc.shadowParams.w), inb);
}
@vertex fn vs(@location(0) pos: vec3f, @location(1) norm: vec3f, @location(2) fromPos: vec3f) -> V {
let local = mix(fromPos, pos, ob.morph.x);
let wp = (ob.model * vec4f(local,1)).xyz;
let wn = normalize((ob.nmat * vec4f(norm,0)).xyz);
return V(sc.vp * vec4f(wp,1), wn, wp);
}
@fragment fn fs(v: V) -> @location(0) vec4f {
if (sc.options.x > 0.5 && dot(v.w, sc.section.xyz) < sc.section.w) { discard; }
let shadow = shadowFactor(v.w);
if (sc.options.x > 0.5 && dot(v.w, sc.section.xyz) < sc.section.w + 0.02) { return vec4f(mix(ob.baseColor, sc.capColor, 0.6) + ob.emissive * 0.2, ob.style.x); }
let N = normalize(v.n);
let L = normalize(sc.light.xyz);
let V2 = normalize(sc.eye.xyz - v.w);
let selected = mix(ob.color.rgb, sc.selectionColor, ob.style.y * 0.48);
let base = mix(selected, sc.hoverColor, ob.style.w * 0.38) * ob.baseColor;
let d = max(dot(N, L), 0.0);
let step = clamp(floor(d * 4.0) / 4.0 + 0.12, 0.0, 1.0);
let rim = pow(1.0 - clamp(abs(dot(N, V2)), 0.0, 1.0), 3.0);
let outline = 1.0 - smoothstep(0.55, 0.95, rim) * 0.85;
let c = base * (sc.ambient.rgb * 0.6 * shadow + step) * outline + ob.emissive;
return vec4f(c, ob.style.x);
}`,!1,!1,!0],[`meshUnlit`,`struct Scene { vp: mat4x4f, eye: vec4f, light: vec4f, ambient: vec4f, section: vec4f, options: vec4f, inverseVP: mat4x4f, selectionColor: vec3f, hoverColor: vec3f, edgeColor: vec3f, xrayColor: vec3f, gridColor: vec3f, capColor: vec3f, lightVP: mat4x4f, shadowParams: vec4f }
struct Obj { model: mat4x4f, nmat: mat4x4f, color: vec4f, style: vec4f, morph: vec4f, baseColor: vec3f, metallic: f32, emissive: vec3f, roughness: f32, materialId: f32 }
@group(0) @binding(0) var<uniform> sc: Scene;
@group(1) @binding(0) var<uniform> ob: Obj;
struct V { @builtin(position) p: vec4f, @location(0) n: vec3f, @location(1) w: vec3f }
@vertex fn vs(@location(0) pos: vec3f, @location(1) norm: vec3f, @location(2) fromPos: vec3f) -> V {
let local = mix(fromPos, pos, ob.morph.x);
let wp = (ob.model * vec4f(local,1)).xyz;
let wn = normalize((ob.nmat * vec4f(norm,0)).xyz);
return V(sc.vp * vec4f(wp,1), wn, wp);
}
@fragment fn fs(v: V) -> @location(0) vec4f {
if (sc.options.x > 0.5 && dot(v.w, sc.section.xyz) < sc.section.w) { discard; }
if (sc.options.x > 0.5 && dot(v.w, sc.section.xyz) < sc.section.w + 0.02) { return vec4f(mix(ob.baseColor, sc.capColor, 0.6) + ob.emissive * 0.2, ob.style.x); }
let selected = mix(ob.color.rgb, sc.selectionColor, ob.style.y * 0.48);
let base = mix(selected, sc.hoverColor, ob.style.w * 0.38) * ob.baseColor;
return vec4f(base + ob.emissive, ob.style.x);
}`,!1,!1,!0]])o({id:e,source:t,kind:`object`,blend:`none`,depth:{writeEnabled:!0,compare:`less`},topology:`triangle-list`,cullMode:`none`,vertexLayout:`mesh`,supportsVariants:i,usesMatcapBinding:n,usesEnvBinding:r,usesShadowBinding:e!==`meshUnlit`});o({id:`meshSectionCap`,source:`struct Scene { vp: mat4x4f, eye: vec4f, light: vec4f, ambient: vec4f, section: vec4f, options: vec4f, inverseVP: mat4x4f, selectionColor: vec3f, hoverColor: vec3f, edgeColor: vec3f, xrayColor: vec3f, gridColor: vec3f, capColor: vec3f, lightVP: mat4x4f, shadowParams: vec4f }
struct Obj { model: mat4x4f, nmat: mat4x4f, color: vec4f, style: vec4f, morph: vec4f, baseColor: vec3f, metallic: f32, emissive: vec3f, roughness: f32, materialId: f32 }
@group(0) @binding(0) var<uniform> sc: Scene;
@group(1) @binding(0) var<uniform> ob: Obj;
struct V { @builtin(position) p: vec4f, @location(0) n: vec3f, @location(1) w: vec3f }
@vertex fn vs(@location(0) pos: vec3f, @location(1) norm: vec3f, @location(2) fromPos: vec3f) -> V {
let local = mix(fromPos, pos, ob.morph.x);
let wp = (ob.model * vec4f(local,1)).xyz;
let wn = normalize((ob.nmat * vec4f(norm,0)).xyz);
return V(sc.vp * vec4f(wp,1), wn, wp);
}
@fragment fn fs(v: V) -> @location(0) vec4f {
if (sc.options.x > 0.5 && dot(v.w, sc.section.xyz) >= sc.section.w) { discard; }
return vec4f(sc.capColor, 1.0);
}`,kind:`object`,blend:`none`,depth:{writeEnabled:!0,compare:`less`},topology:`triangle-list`,cullMode:`front`,vertexLayout:`mesh`,supportsVariants:!0}),o({id:`deepMesh`,source:`struct Scene { vp: mat4x4f, eye: vec4f, light: vec4f, ambient: vec4f, section: vec4f, options: vec4f, inverseVP: mat4x4f, selectionColor: vec3f, hoverColor: vec3f, edgeColor: vec3f, xrayColor: vec3f, gridColor: vec3f, capColor: vec3f, lightVP: mat4x4f, shadowParams: vec4f }
struct Obj { model: mat4x4f, nmat: mat4x4f, color: vec4f, style: vec4f, morph: vec4f, baseColor: vec3f, metallic: f32, emissive: vec3f, roughness: f32, materialId: f32 }
@group(0) @binding(0) var<uniform> sc: Scene;
@group(1) @binding(0) var<uniform> ob: Obj;
struct V { @builtin(position) p: vec4f, @location(0) w: vec3f, @location(1) n: vec3f }
@vertex fn vs(@location(0) pos: vec3f, @location(1) norm: vec3f, @location(2) fromPos: vec3f) -> V {
let local = mix(fromPos, pos, ob.morph.x);
let world = (ob.model * vec4f(local, 1)).xyz;
let wn = normalize((ob.nmat * vec4f(norm, 0)).xyz);
return V(sc.vp * vec4f(world, 1), world, wn);
}
@fragment fn fs(v: V) -> @location(0) vec4f {
if (sc.options.x > 0.5 && dot(v.w, sc.section.xyz) < sc.section.w) { discard; }
let N = normalize(v.n);
let V2 = normalize(sc.eye.xyz - v.w);
let fresnel = pow(1.0 - clamp(abs(dot(N, V2)), 0.0, 1.0), 2.0);
let glow = sc.xrayColor * (0.85 + 0.55 * fresnel);
return vec4f(glow, mix(0.1, 0.55, fresnel));
}`,kind:`object`,blend:`alpha`,depth:{writeEnabled:!1,compare:`always`},topology:`triangle-list`,cullMode:`none`,vertexLayout:`mesh`,supportsVariants:!0}),o({id:`edge`,source:`struct Scene { vp: mat4x4f, eye: vec4f, light: vec4f, ambient: vec4f, section: vec4f, options: vec4f, inverseVP: mat4x4f, selectionColor: vec3f, hoverColor: vec3f, edgeColor: vec3f, xrayColor: vec3f, gridColor: vec3f, capColor: vec3f, lightVP: mat4x4f, shadowParams: vec4f }
struct Obj { model: mat4x4f, nmat: mat4x4f, color: vec4f, style: vec4f, morph: vec4f, baseColor: vec3f, metallic: f32, emissive: vec3f, roughness: f32, materialId: f32 }
@group(0) @binding(0) var<uniform> sc: Scene;
@group(1) @binding(0) var<uniform> ob: Obj;
struct EdgeV { @builtin(position) p: vec4f, @location(0) w: vec3f }
@vertex fn vs(@location(0) pos: vec3f, @location(1) fromPos: vec3f) -> EdgeV {
let local = mix(fromPos, pos, ob.morph.x);
let world = (ob.model * vec4f(local, 1)).xyz;
var p = sc.vp * ob.model * vec4f(local, 1);
p.z -= 0.00008 * p.w;
return EdgeV(p, world);
}
@fragment fn fs(v: EdgeV) -> @location(0) vec4f {
if (sc.options.x > 0.5 && dot(v.w, sc.section.xyz) < sc.section.w) { discard; }
let selected = mix(sc.edgeColor, vec3f(1.0, 0.55, 0.08), ob.style.y);
let color = mix(selected, vec3f(0.1, 0.82, 1.0), ob.style.w);
return vec4f(color, ob.style.z);
}`,kind:`object`,blend:`alpha`,depth:{writeEnabled:!1,compare:`less-equal`},topology:`line-list`,cullMode:`none`,vertexLayout:`edge`,supportsVariants:!0}),o({id:`line`,source:`struct Scene { vp: mat4x4f, eye: vec4f, light: vec4f, ambient: vec4f, section: vec4f, options: vec4f }
@group(0) @binding(0) var<uniform> sc: Scene;
struct V { @builtin(position) p: vec4f, @location(0) c: vec4f }
@vertex fn vs(@location(0) pos: vec3f, @location(1) col: vec4f) -> V {
return V(sc.vp * vec4f(pos,1), col);
}
@fragment fn fs(v: V) -> @location(0) vec4f { return v.c; }`,kind:`line`,blend:`alpha`,depth:{writeEnabled:!0,compare:`less`},topology:`line-list`,cullMode:`none`,vertexLayout:`line`,supportsVariants:!1}),o({id:`grid`,source:`struct Scene { vp: mat4x4f, eye: vec4f, light: vec4f, ambient: vec4f, section: vec4f, options: vec4f, inverseVP: mat4x4f, selectionColor: vec3f, hoverColor: vec3f, edgeColor: vec3f, xrayColor: vec3f, gridColor: vec3f, capColor: vec3f, lightVP: mat4x4f, shadowParams: vec4f }
@group(0) @binding(0) var<uniform> sc: Scene;
@group(1) @binding(0) var shadowMap: texture_depth_2d;
@group(1) @binding(1) var shadowSampler: sampler_comparison;
fn shadowFactor(wp: vec3f) -> f32 {
if (sc.shadowParams.x < 0.5) { return 1.0; }
let lp = sc.lightVP * vec4f(wp, 1.0);
let ndc = lp.xyz / lp.w;
let uv = ndc.xy * vec2f(0.5, -0.5) + vec2f(0.5);
let inb = ndc.z >= 0.0 && ndc.z <= 1.0 && uv.x >= 0.0 && uv.x <= 1.0 && uv.y >= 0.0 && uv.y <= 1.0;
let texel = sc.shadowParams.y;
let depth = ndc.z - sc.shadowParams.z;
var sum = 0.0;
for (var dy = -1; dy <= 1; dy++) {
for (var dx = -1; dx <= 1; dx++) {
sum += textureSampleCompare(shadowMap, shadowSampler, clamp(uv, vec2f(0.0), vec2f(1.0)) + vec2f(f32(dx) * texel, f32(dy) * texel), depth);
}
}
return select(1.0, mix(1.0, sum / 9.0, sc.shadowParams.w), inb);
}
struct V { @builtin(position) p: vec4f, @location(0) near: vec4f, @location(1) far: vec4f }
struct F { @location(0) color: vec4f, @builtin(frag_depth) depth: f32 }
@vertex fn vs(@location(0) corner: vec2f) -> V {
return V(vec4f(corner, 0.0, 1.0),
sc.inverseVP * vec4f(corner, 0.0, 1.0),
sc.inverseVP * vec4f(corner, 1.0, 1.0));
}
fn lineMask(coord: vec2f, width: vec2f) -> f32 {
let g = abs(fract(coord - 0.5) - 0.5) / max(width, vec2f(0.000001));
return 1.0 - min(min(g.x, g.y), 1.0);
}
@fragment fn fs(v: V) -> F {
let near = v.near.xyz / v.near.w;
let far = v.far.xyz / v.far.w;
let ray = far - near;
let dz = select(-1.0, 1.0, ray.z >= 0.0) * max(abs(ray.z), 0.000001);
let t = -near.z / dz;
let world = near + t * ray;
let pixelWidth = max(fwidth(world.xy), vec2f(0.000001));
let level = max(0.0, log2(max(pixelWidth.x, pixelWidth.y) * 8.0 / sc.options.y) / log2(10.0));
let step = sc.options.y * pow(10.0, floor(level));
let blend = fract(level);
let minor = lineMask(world.xy / step, pixelWidth / step);
let major = lineMask(world.xy / (step * 10.0), pixelWidth / (step * 10.0));
let coarse = lineMask(world.xy / (step * 100.0), pixelWidth / (step * 100.0));
var alpha = max(max(minor * 0.28 * (1.0 - blend), major * mix(0.55, 0.28, blend)), coarse * 0.55 * blend);
var color = sc.gridColor;
let shadow = shadowFactor(vec3f(world.xy, 0.02));
color = color * shadow;
let axisW = pixelWidth * 1.2;
if (abs(world.y) < axisW.y) { color = vec3f(0.95, 0.18, 0.16); alpha = max(alpha, 0.9); }
if (abs(world.x) < axisW.x) { color = vec3f(0.2, 0.85, 0.25); alpha = max(alpha, 0.9); }
let clip = sc.vp * vec4f(world.xy, 0.0, 1.0);
let depth = clip.z / clip.w;
if (abs(ray.z) < 0.000001 || t < 0.0 || depth < 0.0 || alpha < 0.004) { discard; }
return F(vec4f(color, alpha), min(depth, 0.999999));
}`,kind:`grid`,blend:`alpha`,depth:{writeEnabled:!0,compare:`less`},topology:`triangle-list`,cullMode:`none`,vertexLayout:`grid`,supportsVariants:!1,usesShadowBinding:!0}),o({id:`meshShadow`,source:`struct Scene { vp: mat4x4f, eye: vec4f, light: vec4f, ambient: vec4f, section: vec4f, options: vec4f, inverseVP: mat4x4f, selectionColor: vec3f, hoverColor: vec3f, edgeColor: vec3f, xrayColor: vec3f, gridColor: vec3f, capColor: vec3f, lightVP: mat4x4f, shadowParams: vec4f }
struct Obj { model: mat4x4f, nmat: mat4x4f, color: vec4f, style: vec4f, morph: vec4f, baseColor: vec3f, metallic: f32, emissive: vec3f, roughness: f32, materialId: f32 }
@group(0) @binding(0) var<uniform> sc: Scene;
@group(1) @binding(0) var<uniform> ob: Obj;
@vertex fn vs(@location(0) pos: vec3f, @location(1) norm: vec3f, @location(2) fromPos: vec3f) -> @builtin(position) vec4f {
let local = mix(fromPos, pos, ob.morph.x);
return sc.lightVP * (ob.model * vec4f(local, 1.0));
}`,kind:`object`,blend:`none`,depth:{writeEnabled:!0,compare:`less`},topology:`triangle-list`,cullMode:`none`,vertexLayout:`mesh`,supportsVariants:!1,depthOnly:!0}),o({id:`selectionOverlay`,source:`struct Scene { vp: mat4x4f, eye: vec4f, light: vec4f, ambient: vec4f, section: vec4f, options: vec4f }
@group(0) @binding(0) var<uniform> sc: Scene;
struct V { @builtin(position) p: vec4f, @location(0) c: vec4f, @location(1) w: vec3f }
@vertex fn vs(@location(0) pos: vec3f, @location(1) col: vec4f) -> V {
var clip = sc.vp * vec4f(pos, 1);
clip.z -= 0.00018 * clip.w;
return V(clip, col, pos);
}
@fragment fn fs(v: V) -> @location(0) vec4f {
if (sc.options.x > 0.5 && dot(v.w, sc.section.xyz) < sc.section.w) { discard; }
return v.c;
}`,kind:`overlay`,blend:`alpha`,depth:{writeEnabled:!1,compare:`less-equal`},topology:`triangle-list`,cullMode:`none`,vertexLayout:`line`,supportsVariants:!1});export{t as a,r as i,n,e as o,i as r,s as t};