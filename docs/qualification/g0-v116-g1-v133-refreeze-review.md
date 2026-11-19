# G0 v116 and G1 v133

Rebuild the WASM kernels from the merged PR 66 source tree. Bind the Ubuntu runner image observed in [job 135621854261](https://github.com/themoretheless/open-scad-viewer/actions/runs/44541500792/job/135621854261). The successor plan requires all 4,740 work units to run again.

The [macOS preflight failure](https://github.com/themoretheless/open-scad-viewer/actions/runs/44541500792/job/135621879018) identified image 20261118.0550.1. The [Windows checkout failure](https://github.com/themoretheless/open-scad-viewer/actions/runs/44541500792/job/135621881677) occurred before preflight because the qualification archive contains long paths. Enable Git long paths before checkout.
