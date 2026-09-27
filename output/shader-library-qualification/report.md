# Shader library browser qualification

- Date: 2026-09-26T23:14:07.493Z
- Adapter: {}
- Immediate variant active: false
- Overall (stock sources): PASS

## Stock sources (as committed)

### Shader module compilation (Tint), shader × variant

| shader/variant | diagnostics |
| --- | --- |
| mesh/uniform | ok |
| mesh/immediate | ok |
| mesh/instanced | ok |
| meshPbr/uniform | ok |
| meshPbr/immediate | ok |
| meshPbr/instanced | ok |
| meshMatcap/uniform | ok |
| meshMatcap/immediate | ok |
| meshMatcap/instanced | ok |
| meshToon/uniform | ok |
| meshToon/immediate | ok |
| meshToon/instanced | ok |
| meshUnlit/uniform | ok |
| meshUnlit/immediate | ok |
| meshUnlit/instanced | ok |

### Pipeline creation

| pipeline | result |
| --- | --- |
| mesh/uniform | ok |
| mesh/immediate | ok |
| mesh/instanced | ok |
| meshPbr/uniform | ok |
| meshPbr/immediate | unsupported |
| meshPbr/instanced | unsupported |
| meshMatcap/uniform | ok |
| meshMatcap/immediate | unsupported |
| meshMatcap/instanced | unsupported |
| meshToon/uniform | ok |
| meshToon/immediate | ok |
| meshToon/instanced | ok |
| meshUnlit/uniform | ok |
| meshUnlit/immediate | ok |
| meshUnlit/instanced | ok |

### Configurations (stock sources)

| config | stddev | non-bg % | result | screenshot |
| --- | --- | --- | --- | --- |
| model-phong | 26.6 | 25.2 | PASS | stock-model-phong.png |
| model-pbr | 32.3 | 25.2 | PASS | stock-model-pbr.png |
| model-matcap | 37.3 | 25.2 | PASS | stock-model-matcap.png |
| model-toon | 22.9 | 25.2 | PASS | stock-model-toon.png |
| model-unlit | 45.0 | 25.2 | PASS | stock-model-unlit.png |
| model-pbr-metal | 29.1 | 25.2 | PASS | stock-model-pbr-metal.png |
| matcap-procedural | 35.1 | 25.2 | PASS | stock-matcap-procedural.png |
| matcap-studio | 33.4 | 25.2 | PASS | stock-matcap-studio.png |
| matcap-clay | 30.8 | 25.2 | PASS | stock-matcap-clay.png |
| matcap-chrome | 23.2 | 25.2 | PASS | stock-matcap-chrome.png |
| matcap-pearl | 22.5 | 25.2 | PASS | stock-matcap-pearl.png |
| env-none | 29.1 | 25.2 | PASS | stock-env-none.png |
| env-studio-softbox | 33.7 | 25.2 | PASS | stock-env-studio-softbox.png |
| env-outdoor-sky | 33.5 | 25.2 | PASS | stock-env-outdoor-sky.png |
| env-workshop | 33.0 | 25.2 | PASS | stock-env-workshop.png |
| shadows-off | 26.0 | 25.2 | PASS | stock-shadows-off.png |
| shadows-on | 26.2 | 25.0 | PASS | stock-shadows-on.png |
| shadows-on-pbr-env | 32.8 | 25.0 | PASS | stock-shadows-on-pbr-env.png |
| theme-default | 26.0 | 25.2 | PASS | stock-theme-default.png |
| theme-dark-contrast | 29.2 | 26.0 | PASS | stock-theme-dark-contrast.png |
| theme-light | 57.4 | 25.2 | PASS | stock-theme-light.png |
| section-off | 26.0 | 25.2 | PASS | stock-section-off.png |
| section-on | 41.8 | 25.2 | PASS | stock-section-on.png |
| section-on-toon | 40.2 | 25.2 | PASS | stock-section-on-toon.png |
| instanced-phong | 30.1 | 30.0 | PASS | stock-instanced-phong.png |
| instanced-toon | 26.3 | 30.0 | PASS | stock-instanced-toon.png |

### Status events

- none

## Patched sources (in-memory uniformity repair)

The patch rewrites shadowFactor's non-uniform bounds early-return as a
clamped select and hoists the shadowFactor call above the section-cap
early return; nothing on disk is modified.

### Shader module compilation (Tint), shader × variant

| shader/variant | diagnostics |
| --- | --- |
| mesh/uniform | ok |
| mesh/immediate | ok |
| mesh/instanced | ok |
| meshPbr/uniform | ok |
| meshPbr/immediate | ok |
| meshPbr/instanced | ok |
| meshMatcap/uniform | ok |
| meshMatcap/immediate | ok |
| meshMatcap/instanced | ok |
| meshToon/uniform | ok |
| meshToon/immediate | ok |
| meshToon/instanced | ok |
| meshUnlit/uniform | ok |
| meshUnlit/immediate | ok |
| meshUnlit/instanced | ok |

### Pipeline creation

| pipeline | result |
| --- | --- |
| mesh/uniform | ok |
| mesh/immediate | ok |
| mesh/instanced | ok |
| meshPbr/uniform | ok |
| meshPbr/immediate | unsupported |
| meshPbr/instanced | unsupported |
| meshMatcap/uniform | ok |
| meshMatcap/immediate | unsupported |
| meshMatcap/instanced | unsupported |
| meshToon/uniform | ok |
| meshToon/immediate | ok |
| meshToon/instanced | ok |
| meshUnlit/uniform | ok |
| meshUnlit/immediate | ok |
| meshUnlit/instanced | ok |

### Configurations

| config | stddev | non-bg % | result | screenshot |
| --- | --- | --- | --- | --- |
| model-phong | 26.6 | 25.2 | PASS | model-phong.png |
| model-pbr | 32.3 | 25.2 | PASS | model-pbr.png |
| model-matcap | 37.3 | 25.2 | PASS | model-matcap.png |
| model-toon | 22.9 | 25.2 | PASS | model-toon.png |
| model-unlit | 45.0 | 25.2 | PASS | model-unlit.png |
| model-pbr-metal | 29.1 | 25.2 | PASS | model-pbr-metal.png |
| matcap-procedural | 35.1 | 25.2 | PASS | matcap-procedural.png |
| matcap-studio | 33.4 | 25.2 | PASS | matcap-studio.png |
| matcap-clay | 30.8 | 25.2 | PASS | matcap-clay.png |
| matcap-chrome | 23.2 | 25.2 | PASS | matcap-chrome.png |
| matcap-pearl | 22.5 | 25.2 | PASS | matcap-pearl.png |
| env-none | 29.1 | 25.2 | PASS | env-none.png |
| env-studio-softbox | 33.7 | 25.2 | PASS | env-studio-softbox.png |
| env-outdoor-sky | 33.5 | 25.2 | PASS | env-outdoor-sky.png |
| env-workshop | 33.0 | 25.2 | PASS | env-workshop.png |
| shadows-off | 26.0 | 25.2 | PASS | shadows-off.png |
| shadows-on | 26.2 | 25.0 | PASS | shadows-on.png |
| shadows-on-pbr-env | 32.8 | 25.0 | PASS | shadows-on-pbr-env.png |
| theme-default | 26.0 | 25.2 | PASS | theme-default.png |
| theme-dark-contrast | 29.2 | 26.0 | PASS | theme-dark-contrast.png |
| theme-light | 57.4 | 25.2 | PASS | theme-light.png |
| section-off | 26.0 | 25.2 | PASS | section-off.png |
| section-on | 41.8 | 25.2 | PASS | section-on.png |
| section-on-toon | 40.2 | 25.2 | PASS | section-on-toon.png |
| instanced-phong | 30.1 | 30.0 | PASS | instanced-phong.png |
| instanced-toon | 26.3 | 30.0 | PASS | instanced-toon.png |

### Status events

- none
