# Shader library browser qualification

- Date: 2026-09-26T18:38:50.865Z
- Adapter: {}
- Immediate variant active: true
- Overall: FAIL

## Pipeline creation (shading model × variant)

| pipeline | result |
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

## Configurations

| config | stddev | non-bg % | result | screenshot |
| --- | --- | --- | --- | --- |
| model-phong | 0.0 | 0.0 | FAIL | model-phong.png |
| model-pbr | 0.0 | 0.0 | FAIL | model-pbr.png |
| model-matcap | 0.0 | 0.0 | FAIL | model-matcap.png |
| model-toon | 0.0 | 0.0 | FAIL | model-toon.png |
| model-unlit | 0.0 | 0.0 | FAIL | model-unlit.png |
| model-pbr-metal | 0.0 | 0.0 | FAIL | model-pbr-metal.png |
| matcap-procedural | 0.0 | 0.0 | FAIL | matcap-procedural.png |
| matcap-studio | 0.0 | 0.0 | FAIL | matcap-studio.png |
| matcap-clay | 0.0 | 0.0 | FAIL | matcap-clay.png |
| matcap-chrome | 0.0 | 0.0 | FAIL | matcap-chrome.png |
| matcap-pearl | 0.0 | 0.0 | FAIL | matcap-pearl.png |
| env-none | 0.0 | 0.0 | FAIL | env-none.png |
| env-studio-softbox | 0.0 | 0.0 | FAIL | env-studio-softbox.png |
| env-outdoor-sky | 0.0 | 0.0 | FAIL | env-outdoor-sky.png |
| env-workshop | 0.0 | 0.0 | FAIL | env-workshop.png |
| shadows-off | 0.0 | 0.0 | FAIL | shadows-off.png |
| shadows-on | 0.0 | 0.0 | FAIL | shadows-on.png |
| shadows-on-pbr-env | 0.0 | 0.0 | FAIL | shadows-on-pbr-env.png |
| theme-default | 0.0 | 0.0 | FAIL | theme-default.png |
| theme-dark-contrast | 0.0 | 0.0 | FAIL | theme-dark-contrast.png |
| theme-light | 0.0 | 0.0 | FAIL | theme-light.png |
| section-off | 0.0 | 0.0 | FAIL | section-off.png |
| section-on | 0.0 | 0.0 | FAIL | section-on.png |
| section-on-toon | 0.0 | 0.0 | FAIL | section-on-toon.png |
| instanced-phong | 0.0 | 0.0 | FAIL | instanced-phong.png |
| instanced-toon | 0.0 | 0.0 | FAIL | instanced-toon.png |
