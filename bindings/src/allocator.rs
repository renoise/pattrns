use core::ffi::c_void;

// -------------------------------------------------------------------------------------------------

pub type AllocFn = extern "C" fn(u32, u32) -> *mut c_void;
pub type DeallocFn = extern "C" fn(*mut c_void, u32, u32) -> ();

// -------------------------------------------------------------------------------------------------

// we either use a dhat-profiler or an external allocator or the default one
#[cfg(feature = "dhat-profiler")]
mod dhat;
#[cfg(not(feature = "dhat-profiler"))]
mod external;
