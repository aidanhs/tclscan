#![feature(slicing_syntax)]
#![feature(globs)]

extern crate libc;

use std::io::File;

pub mod rstcl;
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
        Ok(v) => scan_script(v.as_slice()),
        Err(e) => println!("WARN: Couldn't read {}: {}", path, e),
    }
}

fn is_literal(token: &rstcl::TclToken) -> bool {
    let token_str = token.val;
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

fn is_safe_val(token: &rstcl::TclToken) -> bool {
    let token_str = token.val;
    assert!(token_str.len() > 0);
    if is_literal(token) {
        return true;
    }
    return false;
}
#[deriving(Clone)]
enum Code {
    Block,
    Expr,
    Literal,
    Normal,
}

/// Checks if a parsed command is insecure
///
/// ```
/// use tclscan;
/// use tclscan::rstcl;
/// fn check(cmd: &str, insecure: bool) {
///     let (parse, _) = rstcl::parse_command(cmd);
///     assert!(Ok(insecure) == tclscan::is_command_insecure(parse.tokens));
/// }
/// check("puts x", false);
/// check("puts [x]", false);
/// check("expr {[blah]}", false);
/// check("expr \"[blah]\"", true);
/// // check("if [info exists abc] {}", false);
/// // check("expr {[expr \"[blah]\"]}", true);
/// ```
pub fn is_command_insecure(tokens: Vec<rstcl::TclToken>) -> Result<bool, &str> {
    let param_types = match tokens[0].val {
        // eval script
        "eval" => Vec::from_elem(tokens.len()-1, Code::Block),
        // catch script [result]? [options]?
        "catch" => {
            let mut param_types = vec![Code::Block];
            if tokens.len() == 3 || tokens.len() == 4 {
                param_types.push_all(Vec::from_elem(tokens.len()-2, Code::Literal).as_slice());
            }
            param_types
        }
        // expr [arg]+
        "expr" => tokens[1..].iter().map(|_| Code::Expr).collect(),
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
            while i < tokens.len() {
                param_types.push_all(match tokens[i].val {
                    "elseif" => vec![Code::Literal, Code::Expr, Code::Block],
                    "else" => vec![Code::Literal, Code::Block],
                    _ => { break; },
                }.as_slice());
                i = param_types.len() + 1;
            }
            param_types
        },
        _ => Vec::from_elem(tokens.len()-1, Code::Normal),
    };
    if param_types.len() != tokens.len() - 1 {
        return Err("badly formed command");
    }
    let mut insecure = false;
    for (param_type, param) in param_types.iter().zip(tokens[1..].iter()) {
        insecure = insecure || match *param_type {
            Code::Block => scan_block(param),
            Code::Expr => !is_literal(param),
            Code::Literal => !is_literal(param),
            Code::Normal => false,
        }
    }
    return Ok(insecure);
}

/// Scans a block (i.e. should be quoted) for danger
fn scan_block<'a>(token: &rstcl::TclToken) -> bool {
    let block_str = token.val;
    if !(block_str.starts_with("{") && block_str.ends_with("}")) {
        println!("WARN: Not a block {}", block_str);
        return !is_safe_val(token);
    }
    let script_str = block_str[1..block_str.len()-1];
    scan_script(script_str);
    return false;
}

/// Scans a sequence of commands for danger
fn scan_script<'a>(string: &'a str) {
    let mut script: &'a str = string;
    while script.len() > 0 {
        let (parse, remaining) = rstcl::parse_command(script);
        script = remaining;
        if parse.tokens.len() == 0 {
            continue;
        }
        let command = parse.command;
        match is_command_insecure(parse.tokens) {
            Ok(true) => println!("DANGER: {}", command),
            Ok(false) => (),
            Err(e) => println!("WARN: {}", e),
        }
    }
}
