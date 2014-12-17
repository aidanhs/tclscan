#![feature(slicing_syntax)]
extern crate libc;

use std::io::File;
use std::mem::uninitialized;

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
        Ok(v) => scancontents(v.as_slice()),
        Err(e) => println!("WARN: Couldn't read {}: {}", path, e),
    }
}

fn scancontents<'a>(contents: &'a str) {
    let mut script: &'a str = contents;
    while script.len() > 0 {
        let (comment, command, remaining) = parsecommand(script);
        script = remaining;
        println!(">{}< >{}<", comment, command);
    }
}

fn parsecommand<'a>(script: &'a str/*, nested*/) -> (&'a str, &'a str, &'a str) {
    unsafe {
        let mut parse: tcl::Tcl_Parse = uninitialized();
        let parse_ptr: *mut tcl::Tcl_Parse = &mut parse;

        // https://github.com/rust-lang/rust/issues/16035
        let script_cstr = script.to_c_str();
        let script_ptr = script_cstr.as_ptr();
        let script_start = script_ptr as uint;

        // interp, start, numBytes, nested, parsePtr
        assert!(tcl::Tcl_ParseCommand(I.unwrap(), script_ptr, -1, 0, parse_ptr) == 0);
        let token_strs = gettokens(script, &parse, script_start);

        // commentStart seems to be undefined if commentSize == 0
        let comment = match parse.commentSize.to_uint().unwrap() {
            0 => "",
            l => {
                let offset = parse.commentStart as uint - script_start;
                script[offset..offset+l]
            },
        };
        let command_len = parse.commandSize.to_uint().unwrap();
        let command_off = parse.commandStart as uint - script_start;
        let command = script[command_off..command_off+command_len];
        let remaining = script[command_off+command_len..];

        tcl::Tcl_FreeParse(parse_ptr);
        return (comment, command, remaining);
    }
}

unsafe fn gettokens<'a>(script: &'a str, parse: &tcl::Tcl_Parse, script_start: uint) -> Vec<&'a str> {
    let num = parse.numTokens as int;
    let token_ptr = parse.tokenPtr;
    let mut token_strs = Vec::new();
    let mut i = 0;
    while i < num {
        let token = *token_ptr.offset(i);
        let offset = token.start as uint - script_start;
        let size = token.size.to_uint().unwrap();
        let token_str = script[offset..offset+size];
        token_strs.push(token_str);
        i += token.numComponents as int;
        i += 1;
    }
    return token_strs;
}
