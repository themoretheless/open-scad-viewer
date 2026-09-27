use crate::CudaError;
use gpu_compute::cuda::cudarc;

const NVRTC_SYMBOLS: &[&str] = &[
    "nvrtcGetNumSupportedArchs",
    "nvrtcGetSupportedArchs",
    "nvrtcCreateProgram",
    "nvrtcCompileProgram",
    "nvrtcDestroyProgram",
    "nvrtcGetPTXSize",
    "nvrtcGetPTX",
    "nvrtcGetProgramLogSize",
    "nvrtcGetProgramLog",
];
const CUBLAS_SYMBOLS: &[&str] = &[
    "cublasCreate_v2",
    "cublasSetStream_v2",
    "cublasDestroy_v2",
    "cublasGemmEx",
];

// cudarc's presence probe only dlopens; its function loader panics when an old
// loadable library lacks a symbol. Check the same retained library first, so
// even panic=abort builds return a useful error before calling into cudarc.
pub(crate) fn require_nvrtc() -> Result<(), CudaError> {
    if !unsafe { cudarc::nvrtc::sys::is_culib_present() } {
        return Err(CudaError::Unavailable(
            "NVRTC library is missing; provide compatible precompiled PTX with from_ptx",
        ));
    }
    validate_symbols("NVRTC", NVRTC_SYMBOLS, |symbol| unsafe {
        cudarc::nvrtc::sys::culib()
            .get::<*const std::ffi::c_void>(symbol.as_bytes())
            .is_ok()
    })
}
pub(crate) fn require_cublas() -> Result<(), CudaError> {
    if !unsafe { cudarc::cublas::sys::is_culib_present() } {
        return Err(CudaError::Unavailable("cuBLAS library is missing"));
    }
    validate_symbols("cuBLAS", CUBLAS_SYMBOLS, |symbol| unsafe {
        cudarc::cublas::sys::culib()
            .get::<*const std::ffi::c_void>(symbol.as_bytes())
            .is_ok()
    })
}
fn validate_symbols(
    library: &'static str,
    symbols: &[&'static str],
    mut available: impl FnMut(&str) -> bool,
) -> Result<(), CudaError> {
    for &symbol in symbols {
        if !available(symbol) {
            return Err(CudaError::MissingSymbol { library, symbol });
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_old_library_returns_its_missing_symbol_before_any_call() {
        let result = validate_symbols("NVRTC", NVRTC_SYMBOLS, |s| s != "nvrtcGetSupportedArchs");
        assert!(matches!(
            result,
            Err(CudaError::MissingSymbol {
                library: "NVRTC",
                symbol: "nvrtcGetSupportedArchs"
            })
        ));
        assert!(validate_symbols("cuBLAS", CUBLAS_SYMBOLS, |_| true).is_ok());
        assert!(matches!(
            validate_symbols("cuBLAS", CUBLAS_SYMBOLS, |s| s != "cublasDestroy_v2"),
            Err(CudaError::MissingSymbol {
                symbol: "cublasDestroy_v2",
                ..
            })
        ));
    }
}
