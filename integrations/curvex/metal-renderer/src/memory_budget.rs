//! Conservative retained-cache budget, not a process or VRAM limit.
use std::{
    sync::{Mutex, OnceLock},
    time::{Duration, Instant},
};
const MIB: u64 = 1024 * 1024;
pub const MAX_CACHE_BYTES: u64 = 512 * MIB;
pub fn budget_for_available(available: Option<u64>) -> u64 {
    available.map_or(64 * MIB, |bytes| {
        bytes
            .saturating_sub(256 * MIB)
            .saturating_div(8)
            .min(MAX_CACHE_BYTES)
    })
}
/// Sample at most once a second; never run an OS query for each shape.
pub fn retained_cache_budget() -> u64 {
    static SAMPLE: OnceLock<Mutex<(Instant, u64)>> = OnceLock::new();
    let sample = SAMPLE
        .get_or_init(|| Mutex::new((Instant::now(), budget_for_available(available_bytes()))));
    let mut sample = sample.lock().unwrap_or_else(|e| e.into_inner());
    if sample.0.elapsed() >= Duration::from_secs(1) {
        *sample = (Instant::now(), budget_for_available(available_bytes()));
    }
    sample.1
}
#[cfg(target_os = "macos")]
#[allow(deprecated)] // libc still exposes the system Mach statistics ABI.
fn available_bytes() -> Option<u64> {
    // Free + inactive pages are a reclaimable estimate; compressed/wired memory
    // is deliberately excluded. A reserve remains for the OS and frame leases.
    unsafe extern "C" {
        fn mach_port_deallocate(
            task: libc::mach_port_t,
            name: libc::mach_port_t,
        ) -> libc::kern_return_t;
    }
    unsafe {
        let host = libc::mach_host_self();
        let mut stats: libc::vm_statistics64 = std::mem::zeroed();
        let mut count = libc::HOST_VM_INFO64_COUNT;
        let result = libc::host_statistics64(
            host,
            libc::HOST_VM_INFO64,
            (&mut stats as *mut libc::vm_statistics64).cast(),
            &mut count,
        );
        let page_size = libc::sysconf(libc::_SC_PAGESIZE);
        mach_port_deallocate(libc::mach_task_self(), host);
        if result != libc::KERN_SUCCESS || page_size <= 0 {
            return None;
        }
        Some(
            (stats.free_count as u64 + stats.inactive_count as u64)
                .saturating_mul(page_size as u64),
        )
    }
}
#[cfg(target_os = "linux")]
fn available_bytes() -> Option<u64> {
    let info = std::fs::read_to_string("/proc/meminfo").ok()?;
    let line = info
        .lines()
        .find(|line| line.starts_with("MemAvailable:"))?;
    line.split_whitespace()
        .nth(1)?
        .parse::<u64>()
        .ok()?
        .checked_mul(1024)
}
#[cfg(not(any(target_os = "macos", target_os = "linux")))]
fn available_bytes() -> Option<u64> {
    None
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn reserve_and_growth_are_bounded() {
        assert_eq!(budget_for_available(None), 64 * MIB);
        assert_eq!(budget_for_available(Some(128 * MIB)), 0);
        assert_eq!(budget_for_available(Some(768 * MIB)), 64 * MIB);
        assert_eq!(budget_for_available(Some(u64::MAX)), MAX_CACHE_BYTES);
    }
    #[test]
    fn native_sample_is_bounded() {
        assert!(retained_cache_budget() <= MAX_CACHE_BYTES);
        #[cfg(target_os = "macos")]
        assert!(available_bytes().is_some());
    }
}
