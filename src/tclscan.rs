extern crate libc;
use libc::c_int;

use std::io::File;

// When https://github.com/crabtw/rust-bindgen/issues/89 is fixed
//#![feature(phase)]
//#[phase(plugin)] extern crate bindgen;
//
//#[allow(dead_code, uppercase_variables, non_camel_case_types)]
//mod tcl_bindings {
//    bindgen!("./mytcl.h", match="tcl.h", link="tclstub")
//}

#[allow(dead_code, non_upper_case_globals, non_camel_case_types, non_snake_case, raw_pointer_deriving)]
mod tcl;

static mut I: Option<*mut tcl::Tcl_Interp> = None;

fn main() {
    unsafe { I = Some(tcl::Tcl_CreateInterp()); }
    unsafe { println!("Tcl_Interp pointer: {}", I); }
    scanfile("testfiles/test.tcl");
}

fn scanfile(path: &str) {
    let mut file = File::open(&Path::new(path));
    match file.read_to_string() {
        Ok(v) => scancontents(v),
        Err(e) => println!("WARN: Couldn't read {}: {}", path, e),
    }
}

fn scancontents(contents: String) {
    println!("{}", contents);
}

fn parsecommand () {}



