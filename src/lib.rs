#![feature(slicing_syntax)]

extern crate libc;

use std::io::File;
use std::iter;
use std::fmt;
use self::CheckResult::*; // TODO: why does swapping this line with one below break?
use rstcl::TokenType;

pub mod rstcl;
#[allow(dead_code, non_upper_case_globals, non_camel_case_types, non_snake_case, raw_pointer_derive)]
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

// TODO: remove show
#[derive(PartialEq, Show)]
pub enum CheckResult {
    Warn(&'static str),
    Danger(&'static str),
}
impl fmt::String for CheckResult {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        return match self {
            &Warn(s) => write!(f, "WARN: {}", s),
            &Danger(s) => write!(f, "DANGER: {}", s),
        };
    }
}

#[derive(Clone)]
enum Code {
    Block,
    Expr,
    Literal,
    Normal,
}

pub fn scan_file(path: &str) {
    let mut file = File::open(&Path::new(path));
    match file.read_to_string() {
        Ok(v) => scan_script(v.as_slice()),
        Err(e) => println!("WARN: Couldn't read {}: {}", path, e),
    }
}

fn check_literal(token: &rstcl::TclToken) -> Vec<CheckResult> {
    let token_str = token.val;
    assert!(token_str.len() > 0);
    return if token_str.char_at(0) == '{' {
        vec![]
    } else if token_str.contains_char('$') {
        vec![Danger("Expected literal, found $")]
    } else if token_str.contains_char('[') {
        vec![Danger("Expected literal, found [")]
    } else {
        vec![]
    }
}

// Does this variable only contain safe characters?
// Only used by is_safe_val
fn is_safe_var(token: &rstcl::TclToken) -> bool {
    assert!(token.ttype == TokenType::Variable);
    return false
}

// Does the return value of this function only contain safe characters?
// Only used by is_safe_val.
fn is_safe_cmd(token: &rstcl::TclToken) -> bool {
    let parse = rstcl::parse_command_token(token);
    let token_strs: Vec<&str> = parse.tokens.iter().map(|e| e.val).collect();
    return match token_strs.as_slice() {
        ["info", "exists", ..] |
        ["catch", ..] => true,
        _ => false,
    };
}

// Check whether a value can ever cause or assist in any security flaw i.e.
// whether it may contain special characters.
// We do *not* concern ourselves with vulnerabilities in sub-commands. That
// should happen elsewhere.
fn is_safe_val(token: &rstcl::TclToken) -> bool {
    assert!(token.val.len() > 0);
    for tok in token.iter() {
        let is_safe = match tok.ttype {
            TokenType::Variable => is_safe_var(tok),
            TokenType::Command => is_safe_cmd(tok),
            _ => true,
        };
        if !is_safe {
            return false;
        }
    }
    return true;
}

/// Checks if a parsed command is insecure
///
/// ```
/// use tclscan::rstcl::parse_command as p;
/// use tclscan::check_command as c;
/// use tclscan::CheckResult::{Danger,Warn};
/// assert!(c(&p("puts x").0.tokens) == vec![]);
/// assert!(c(&p("puts [x]").0.tokens) == vec![]);
/// assert!(c(&p("puts [eval $x]").0.tokens) == vec![Danger("Dangerous unquoted block")]);
/// assert!(c(&p("expr {[blah]}").0.tokens) == vec![]);
/// assert!(c(&p("expr \"[blah]\"").0.tokens) == vec![Danger("Dangerous unquoted expr")]);
/// assert!(c(&p("expr {\\\n0}").0.tokens) == vec![]);
/// assert!(c(&p("expr {[expr \"[blah]\"]}").0.tokens) == vec![Danger("Dangerous unquoted expr")]);
/// assert!(c(&p("if [info exists abc] {}").0.tokens) == vec![Warn("Unquoted expr")]);
/// assert!(c(&p("if [abc] {}").0.tokens) == vec![Danger("Dangerous unquoted expr")]);
/// assert!(c(&p("a${x} blah").0.tokens) == vec![Warn("Non-literal command, cannot scan")]);
/// assert!(c(&p("set a []").0.tokens) == vec![Warn("Non-literal command, cannot scan")]);
/// ```
pub fn check_command(tokens: &Vec<rstcl::TclToken>) -> Vec<CheckResult> {
    let mut results = vec![];
    // First check all subcommands which will be substituted
    for tok in tokens.iter() {
        for subtok in tok.iter().filter(|tok| tok.ttype == TokenType::Command) {
            let parse = rstcl::parse_command_token(subtok);
            results.extend(check_command(&parse.tokens).into_iter());
        }
    }
    // Now check if the command name itself isn't a literal
    if tokens.len() == 0 || check_literal(&tokens[0]).into_iter().len() > 0 {
        results.push(Warn("Non-literal command, cannot scan"));
        return results;
    }
    // Now check the command-specific interpretation of arguments etc
    let param_types = match tokens[0].val {
        // eval script
        "eval" => iter::repeat(Code::Block).take(tokens.len()-1).collect(),
        // catch script [result]? [options]?
        "catch" => {
            let mut param_types = vec![Code::Block];
            if tokens.len() == 3 || tokens.len() == 4 {
                let new_params: Vec<Code> = iter::repeat(Code::Literal).take(tokens.len()-2).collect();
                param_types.push_all(new_params.as_slice());
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
        // while cond body
        "while" => vec![Code::Expr, Code::Block],
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
        _ => iter::repeat(Code::Normal).take(tokens.len()-1).collect(),
    };
    if param_types.len() != tokens.len() - 1 {
        results.push(Warn("badly formed command"));
        return results;
    }
    for (param_type, param) in param_types.iter().zip(tokens[1..].iter()) {
        let check_results: Vec<CheckResult> = match *param_type {
            Code::Block => check_block(param),
            Code::Expr => check_expr(param),
            Code::Literal => check_literal(param),
            Code::Normal => vec![],
        };
        results.extend(check_results.into_iter());
    }
    return results;
}

/// Scans a block (i.e. should be quoted) for danger
fn check_block<'a>(token: &rstcl::TclToken) -> Vec<CheckResult> {
    let block_str = token.val;
    if !(block_str.starts_with("{") && block_str.ends_with("}")) {
        return vec!(match is_safe_val(token) {
            true => Warn("Unquoted block"),
            false => Danger("Dangerous unquoted block"),
        });
    }
    // Block isn't inherently dangerous, let's check functions inside the block
    let script_str = &block_str[1..block_str.len()-1];
    // Note that this is a void return - we don't really want to return all
    // nested issue inside a block as problems of the parent (consider a very
    // long proc).
    scan_script(script_str);
    return vec![];
}

/// Scans an expr (i.e. should be quoted) for danger
fn check_expr<'a>(token: &rstcl::TclToken) -> Vec<CheckResult> {
    let mut results = vec![];
    let expr_str = token.val;
    if !(expr_str.starts_with("{") && expr_str.ends_with("}")) {
        results.push(match is_safe_val(token) {
            true => Warn("Unquoted expr"),
            false => Danger("Dangerous unquoted expr"),
        });
        return results;
    };
    // Technically this is the 'scan_expr' function
    // Expr isn't inherently dangerous, let's check functions inside the expr
    assert!(token.val.starts_with("{") && token.val.ends_with("}"));
    let expr = token.val.slice(1, token.val.len()-1);
    let (parse, remaining) = rstcl::parse_expr(expr);
    assert!(parse.tokens.len() == 1 && remaining == "");
    for tok in parse.tokens[0].iter().filter(|tok| tok.ttype == TokenType::Command) {
        let parse = rstcl::parse_command_token(tok);
        results.extend(check_command(&parse.tokens).into_iter());
    }
    return results;
}

/// Scans a sequence of commands for danger
pub fn scan_script<'a>(string: &'a str) {
    let mut script: &'a str = string;
    while script.len() > 0 {
        let (parse, remaining) = rstcl::parse_command(script);
        script = remaining;
        if parse.tokens.len() == 0 {
            continue;
        }
        match check_command(&parse.tokens).as_slice() {
            [] => (),
            r => {
                println!("COMMAND: {}", parse.command.unwrap().trim_right());
                for check_result in r.iter() {
                    println!("{}", check_result);
                }
                println!("");
            },
        }
    }
}
