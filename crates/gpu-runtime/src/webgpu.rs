#![allow(unsafe_code, dead_code)]

#[derive(Debug, Clone)]
pub struct WebGpuDevice {
    pub name: String,
}

impl WebGpuDevice {
    pub fn probe_available() -> Option<Self> {
        #[cfg(target_arch = "wasm32")]
        {
            Some(Self {
                name: "Browser WebGPU Adapter".to_string(),
            })
        }
        #[cfg(not(target_arch = "wasm32"))]
        {
            None
        }
    }
}
