//extern crate libc;
//use libc::c_int;

// When https://github.com/crabtw/rust-bindgen/issues/89 is fixed
//#![feature(phase)]
//#[phase(plugin)] extern crate bindgen;
//
//#[allow(dead_code, uppercase_variables, non_camel_case_types)]
//mod tcl_bindings {
//    bindgen!("./mytcl.h", match="tcl.h", link="tclstub")
//}

#[repr(C)]
struct Tcl_Interp {
    result_dont_use: *mut u8,                // char *
    free_proc_dont_use: extern fn (*mut u8), // char * -> void
    error_line_dont_use: c_int,              // int
}

#[repr(C)]
struct Tcl_Parse {
    commentStart: *mut u8
}

#[link(name = "tclstub")]
extern {
    fn Tcl_CreateInterp() -> *mut Tcl_Interp;
    fn Tcl_ParseCommand(interp: *mut Tcl_Interp,
                        start: *const u8,
                        numBytes: c_int,
                        nested: c_int,
                        parsePtr: *mut Tcl_Parse) -> u8;
}

fn main() {
    let x = unsafe { Tcl_CreateInterp() };
    println!("max compressed length of a 100 byte buffer: {}", x);
}
