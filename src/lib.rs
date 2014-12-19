#![feature(slicing_syntax)]

extern crate libc;

use std::io::File;

mod rstcl;
#[allow(dead_code, non_upper_case_globals, non_camel_case_types, non_snake_case, raw_pointer_deriving)]
mod tcl;

// http://www.tcl.tk/doc/howto/stubs.html
// Ideally would use stubs but they seem to not work

// When https://github.com/crabtw/rust-bindgen/issues/89 is fixed
//#![feature(phase)]
//#[phase(plugin)] extern crate bindgen;
//
//#[allow(dead_code, uppercase_variables, non_camel_case_types)]
//mod tcl_bindings {
//    bindgen!("./mytcl.h", match="tcl.h", link="tclstub")
//}

pub fn scan_file(path: &str) {
    let mut file = File::open(&Path::new(path));
    match file.read_to_string() {
        Ok(v) => scan_string(v.as_slice()),
        Err(e) => println!("WARN: Couldn't read {}: {}", path, e),
    }
}

fn is_literal(token_str: &str) -> bool {
    assert!(token_str.len() > 0);
    if token_str.char_at(0) == '{' {
        return true;
    }
    if token_str.contains_char('$') {
        return false;
    }
    if token_str.contains_char('[') {
        return false;
    }
    return true;
}

#[deriving(Clone)]
enum Code {
    Block,
    Expr,
    Literal,
    Normal,
}

fn tcltrim(string: &str) -> &str {
    if !(string.starts_with("{") && string.ends_with("}")) {
        println!("WARN: Not a block {}", string);
        return "";
    }
    return string[1..string.len()-1];
}
fn is_command_insecure(token_strs: Vec<&str>) -> Result<bool, &str> {
    let param_types = match token_strs[0] {
        // eval script
        "eval" => Vec::from_elem(token_strs.len()-1, Code::Block),
        // catch script [result]? [options]?
        "catch" => {
            let mut param_types = vec![Code::Block];
            if token_strs.len() == 3 || token_strs.len() == 4 {
                param_types.push_all(Vec::from_elem(token_strs.len()-2, Code::Literal).as_slice());
            }
            param_types
        }
        // expr [arg]+
        "expr" => token_strs[1..].iter().map(|_| Code::Expr).collect(),
        // proc name args body
        "proc" => vec![Code::Literal, Code::Literal, Code::Block],
        // for init cond iter body
        "for" => vec![Code::Block, Code::Expr, Code::Block, Code::Block],
        // foreach [varname list]+ body
        "foreach" => vec![Code::Literal, Code::Normal, Code::Block],
        // if cond body [elseif cond body]* [else body]?
        "if" => {
            let mut param_types = vec![Code::Expr, Code::Block];
            let mut i = 3;
            while i < token_strs.len() {
                param_types.push_all(match token_strs[i] {
                    "elseif" => vec![Code::Literal, Code::Expr, Code::Block],
                    "else" => vec![Code::Literal, Code::Block],
                    _ => { break; },
                }.as_slice());
                i = param_types.len() + 1;
            }
            param_types
        },
        _ => Vec::from_elem(token_strs.len()-1, Code::Normal),
    };
    if param_types.len() != token_strs.len() - 1 {
        return Err("badly formed command");
    }
    let mut insecure = false;
    for (param_type, param) in param_types.iter().zip(token_strs[1..].iter()) {
        insecure = insecure || match *param_type {
            Code::Block => { scan_string(tcltrim(*param)); !is_literal(*param) },
            Code::Expr => !is_literal(*param),
            Code::Literal => !is_literal(*param),
            Code::Normal => false,
        }
    }
    return Ok(insecure);
}

fn scan_string<'a>(string: &'a str) {
    let mut script: &'a str = string;
    while script.len() > 0 {
        let (_, command, token_strs, remaining) = rstcl::parse_command(script);
        script = remaining;
        if token_strs.len() == 0 {
            continue;
        }
        match is_command_insecure(token_strs) {
            Ok(true) => println!("DANGER: {}", command.trim_right_chars('\n')),
            Ok(false) => (),
            Err(e) => println!("WARN: {}", e),
        }
    }
}
