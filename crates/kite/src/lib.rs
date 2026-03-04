use std::ffi::CStr;
use std::os::raw::c_char;

pub fn check_source(source: &str) -> bool {
    kite_core::check_source(source).is_ok()
}

#[no_mangle]
pub extern "C" fn kite_check(source_ptr: *const c_char) -> i32 {
    if source_ptr.is_null() {
        return 1;
    }

    let source = match unsafe { CStr::from_ptr(source_ptr) }.to_str() {
        Ok(source) => source,
        Err(_) => return 1,
    };

    if check_source(source) {
        0
    } else {
        1
    }
}
