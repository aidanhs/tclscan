use std::mem::uninitialized;
use tcl;

static mut I: Option<*mut tcl::Tcl_Interp> = None;
unsafe fn tcl_interp() -> *mut tcl::Tcl_Interp {
    if I.is_none() {
        I = Some(tcl::Tcl_CreateInterp());
    }
    return I.unwrap();
}

pub fn parse_command<'a>(script: &'a str) -> (&'a str, &'a str, Vec<&'a str>, &'a str) {
    unsafe {
        let mut parse: tcl::Tcl_Parse = uninitialized();
        let parse_ptr: *mut tcl::Tcl_Parse = &mut parse;

        // https://github.com/rust-lang/rust/issues/16035
        let script_cstr = script.to_c_str();
        let script_ptr = script_cstr.as_ptr();
        let script_start = script_ptr as uint;

        // interp, start, numBytes, nested, parsePtr
        if tcl::Tcl_ParseCommand(tcl_interp(), script_ptr, -1, 0, parse_ptr) != 0 {
            println!("WARN: couldn't parse {}", script);
            return ("", "", Vec::new(), "");
        }
        let token_strs = get_tokens(script, &parse, script_start);

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
        return (comment, command, token_strs, remaining);
    }
}

unsafe fn get_tokens<'a>(script: &'a str, parse: &tcl::Tcl_Parse, script_start: uint) -> Vec<&'a str> {
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
