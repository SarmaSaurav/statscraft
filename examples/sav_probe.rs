use libloading::{Library, Symbol};
use std::ffi::CString;
use std::os::raw::{c_char, c_int};

fn main() {
    let spssdir = r"C:\Program Files\IBM\SPSS\Statistics\27";
    let old = std::env::var("PATH").unwrap_or_default();
    std::env::set_var("PATH", format!("{};{}", spssdir, old));
    let dll = format!(r"{}\spssio64.dll", spssdir);
    let lib = unsafe { Library::new(&dll).expect("load dll") };
    unsafe {
        let open: Symbol<unsafe extern "C" fn(*const c_char, *mut c_int) -> c_int> =
            lib.get(b"spssOpenRead").unwrap();
        let close: Symbol<unsafe extern "C" fn(c_int) -> c_int> =
            lib.get(b"spssCloseRead").unwrap();
        let nvars: Symbol<unsafe extern "C" fn(c_int, *mut c_int) -> c_int> =
            lib.get(b"spssGetNumberofVariables").unwrap();
        // double-pointer version
        let getnames: Symbol<unsafe extern "C" fn(c_int, *mut *mut c_char, *mut *mut c_int) -> c_int> =
            lib.get(b"spssGetVarNames").unwrap();

        let path = CString::new(r"C:\Program Files\IBM\SPSS\Statistics\27\Samples\English\accidents.sav").unwrap();
        let mut handle: c_int = 0;
        println!("open rc={}", open(path.as_ptr(), &mut handle));
        let mut nv = 0;
        println!("nvars rc={} nv={}", nvars(handle, &mut nv), nv);

        let mut names_ptr: *mut c_char = std::ptr::null_mut();
        let mut types_ptr: *mut c_int = std::ptr::null_mut();
        let rc = getnames(handle, &mut names_ptr, &mut types_ptr);
        println!("getnames rc={} names_ptr={:?} types_ptr={:?}", rc, names_ptr, types_ptr);
        close(handle);
    }
}
