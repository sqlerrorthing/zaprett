use std::ffi::CString;
use std::os::raw::c_char;

pub fn str_to_c_array<const N: usize>(s: &str) -> [c_char; N] {
    let mut array = [0 as c_char; N];

    // Convert &str to CString (adds null terminator)
    let cstr = CString::new(s).unwrap_or_default();
    let bytes = cstr.as_bytes_with_nul();

    let len = bytes.len().min(N);
    for i in 0..len {
        array[i] = bytes[i] as c_char;
    }

    array
}
