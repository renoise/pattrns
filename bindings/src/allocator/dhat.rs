use crate::{
    allocator::{AllocFn, DeallocFn},
    VoidResult,
};

// -------------------------------------------------------------------------------------------------

#[global_allocator]
static DHAT_ALLOCATOR: dhat::Alloc = dhat::Alloc;

// -------------------------------------------------------------------------------------------------

static mut DHAT_PROFILER: Option<dhat::Profiler> = None;

/// cbindgen:ignore
#[no_mangle]
#[allow(clippy::missing_safety_doc)]
pub unsafe extern "C" fn initialize(_alloc: AllocFn, _dealloc: DeallocFn) -> VoidResult {
    // start profiling and ignore external allocator
    DHAT_PROFILER = Some(dhat::Profiler::builder().trim_backtraces(Some(100)).build());
    VoidResult::Ok(())
}

/// cbindgen:ignore
#[no_mangle]
#[allow(clippy::missing_safety_doc)]
pub unsafe extern "C" fn finalize() -> VoidResult {
    // stop profiling
    DHAT_PROFILER = None;
    VoidResult::Ok(())
}
