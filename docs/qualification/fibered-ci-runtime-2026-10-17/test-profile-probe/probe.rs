#[cfg(test)] mod tests {
    #[test] fn optimization_keeps_debug_assertions() {
        assert!(cfg!(debug_assertions));
        assert!(std::panic::catch_unwind(|| debug_assert!(std::hint::black_box(false))).is_err());
    }
    #[test] fn optimization_keeps_overflow_checks() {
        assert!(std::panic::catch_unwind(|| std::hint::black_box(u64::MAX) + 1).is_err());
    }
}
