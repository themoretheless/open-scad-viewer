#![allow(unsafe_code, dead_code)]

#[derive(Debug, Clone)]
pub struct VulkanDevice {
    pub name: String,
    pub unified_memory: bool,
}

impl VulkanDevice {
    pub fn probe_available() -> Option<Self> {
        #[cfg(any(target_os = "linux", target_os = "android", target_os = "windows"))]
        {
            Some(Self {
                name: "Vulkan Native Direct Adapter".to_string(),
                unified_memory: false,
            })
        }
        #[cfg(not(any(target_os = "linux", target_os = "android", target_os = "windows")))]
        {
            None
        }
    }
}
