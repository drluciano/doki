//! C ABI over [`doki_api::handle`] for desktop front ends (the Flutter app
//! on Windows and Linux uses it through Dart FFI).
//!
//! ```c
//! char *doki_call(const char *vault_path, const char *request_json);
//! void doki_free(char *response);
//! ```
//!
//! Both strings are UTF-8 and NUL-terminated. Save folders are local
//! directories opened by the engine itself. The returned string must be
//! released with `doki_free`. Calls block; run them off the UI thread and
//! one at a time per vault. Panics never cross the boundary.

use std::ffi::{c_char, CStr, CString};
use std::panic::{self, AssertUnwindSafe};

use doki_engine::LocalProvider;

fn internal_error(message: &str) -> String {
    let message = message.replace('\\', "\\\\").replace('"', "\\\"");
    format!(
        r#"{{"ok":false,"error":{{"code":"Internal","message":"{message}","save_may_be_mixed":false}}}}"#
    )
}

/// Run one JSON request against the vault at `vault_path`.
///
/// # Safety
/// Both pointers must be valid NUL-terminated strings for the duration of
/// the call.
#[no_mangle]
pub unsafe extern "C" fn doki_call(
    vault_path: *const c_char,
    request: *const c_char,
) -> *mut c_char {
    let response = if vault_path.is_null() || request.is_null() {
        internal_error("null argument")
    } else {
        // SAFETY: the caller guarantees valid NUL-terminated strings.
        let (vault, request) = unsafe { (CStr::from_ptr(vault_path), CStr::from_ptr(request)) };
        match (vault.to_str(), request.to_str()) {
            (Ok(vault), Ok(request)) => panic::catch_unwind(AssertUnwindSafe(|| {
                doki_api::handle(vault, LocalProvider, request)
            }))
            .unwrap_or_else(|_| internal_error("the engine stopped unexpectedly")),
            _ => internal_error("arguments are not UTF-8"),
        }
    };
    // JSON output never contains NUL bytes.
    CString::new(response).map(CString::into_raw).unwrap_or(std::ptr::null_mut())
}

/// Release a string returned by [`doki_call`]. Passing null does nothing.
///
/// # Safety
/// `response` must come from [`doki_call`] and not have been freed already.
#[no_mangle]
pub unsafe extern "C" fn doki_free(response: *mut c_char) {
    if !response.is_null() {
        // SAFETY: allocated by CString::into_raw in doki_call.
        drop(unsafe { CString::from_raw(response) });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trip_and_bad_input() {
        let tmp = tempfile::tempdir().unwrap();
        let vault = CString::new(tmp.path().join("v").to_str().unwrap()).unwrap();
        let call = |req: &str| unsafe {
            let req = CString::new(req).unwrap();
            let out = doki_call(vault.as_ptr(), req.as_ptr());
            let s = CStr::from_ptr(out).to_str().unwrap().to_owned();
            doki_free(out);
            s
        };
        assert!(call(r#"{"cmd":"init","name":"Desk"}"#).starts_with(r#"{"ok":true"#));
        assert!(call(r#"{"cmd":"device"}"#).contains("Desk"));
        assert!(call("nope").contains("InvalidInput"));
        unsafe {
            let out = doki_call(std::ptr::null(), std::ptr::null());
            assert!(CStr::from_ptr(out).to_str().unwrap().contains("Internal"));
            doki_free(out);
        }
    }
}
