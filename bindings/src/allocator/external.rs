use alloc::{alloc::GlobalAlloc, alloc::Layout};
use std::ffi::{CString, c_void};

use crate::{
    allocator::{AllocFn, DeallocFn},
    VoidResult,
};

// -------------------------------------------------------------------------------------------------

/// Allocator which invokes an externally set C allocator
struct ExternalAllocator;

unsafe impl GlobalAlloc for ExternalAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        if let Some(external_alloc) = EXTERNAL_ALLOC {
            external_alloc(layout.size() as u32, layout.align() as u32) as *mut u8
        } else {
            SYSTEM_ALLOCATOR.alloc(layout)
        }
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        if let Some(external_dealloc) = EXTERNAL_DEALLOC {
            external_dealloc(
                ptr as *mut c_void,
                layout.size() as u32,
                layout.align() as u32,
            )
        } else {
            SYSTEM_ALLOCATOR.dealloc(ptr, layout)
        }
    }
}

// -------------------------------------------------------------------------------------------------

#[global_allocator]
static EXTERNAL_ALLOCATOR: ExternalAllocator = ExternalAllocator;
static SYSTEM_ALLOCATOR: std::alloc::System = std::alloc::System;

static mut EXTERNAL_ALLOC: Option<AllocFn> = None;
static mut EXTERNAL_DEALLOC: Option<DeallocFn> = None;

// leaking, do nothing deallocator hook
extern "C" fn leaking_dealloc(_ptr: *mut c_void, _size: u32, _align: u32) {
    // do nothing
}

// -------------------------------------------------------------------------------------------------

#[allow(clippy::missing_safety_doc)]
#[no_mangle]
/// Initialize lib and set external allocator, which should be used instead of the system
/// allocator as global allocator (unless the "dhat-profiler" feature is enabled).
pub unsafe extern "C" fn initialize(alloc: AllocFn, dealloc: DeallocFn) -> VoidResult {
    #[allow(static_mut_refs)]
    if EXTERNAL_ALLOC.is_some() {
        return VoidResult::Error(
            CString::new("pattrns already is initialized.")
                .unwrap()
                .into_raw(),
        );
    }
    EXTERNAL_ALLOC = Some(alloc);
    EXTERNAL_DEALLOC = Some(dealloc);
    VoidResult::Ok(())
}

#[allow(clippy::missing_safety_doc)]
#[no_mangle]
/// Finalize lib: no more calls into the library are allowed after this
pub unsafe extern "C" fn finalize() -> VoidResult {
    #[allow(static_mut_refs)]
    if EXTERNAL_ALLOC.is_none() {
        return VoidResult::Error(
            CString::new("pattrns is not initialized.")
                .unwrap()
                .into_raw(),
        );
    }
    // HACK: just leak when the external allocator no longer is present
    EXTERNAL_ALLOC = None;
    EXTERNAL_DEALLOC = Some(leaking_dealloc);
    VoidResult::Ok(())
}
