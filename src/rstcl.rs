use std::mem::uninitialized;
use std::iter::AdditiveIterator;

use tcl;
use rstcl::TokenType::*;

static mut I: Option<*mut tcl::Tcl_Interp> = None;
unsafe fn tcl_interp() -> *mut tcl::Tcl_Interp {
    if I.is_none() {
        I = Some(tcl::Tcl_CreateInterp());
    }
    return I.unwrap();
}

#[deriving(FromPrimitive, Show, PartialEq)]
pub enum TokenType {
    Word = 1, // TCL_TOKEN_WORD
    SimpleWord = 2, // TCL_TOKEN_SIMPLE_WORD
    Text = 4, // TCL_TOKEN_TEXT
    Bs = 8, // TCL_TOKEN_BS
    Command = 16, // TCL_TOKEN_COMMAND
    Variable = 32, // TCL_TOKEN_VARIABLE
    SubExpr = 64, // TCL_TOKEN_SUB_EXPR
    Operator = 128, // TCL_TOKEN_OPERATOR
    ExpandWord = 256, // TCL_TOKEN_EXPAND_WORD
}

#[deriving(Show, PartialEq)]
pub struct TclParse<'a> {
    pub comment: &'a str,
    pub command: &'a str,
    pub tokens: Vec<TclToken<'a>>,
}
#[deriving(Show, PartialEq)]
pub struct TclToken<'a> {
    pub ttype: TokenType,
    pub val: &'a str,
    pub tokens: Vec<TclToken<'a>>,
}
impl<'b> TclToken<'b> {
    pub fn iter<'a>(&'a self) -> TclTokenIter<'a, 'b> {
        TclTokenIter {
            token: self,
            cur: 0,
        }
    }
    fn traverse(&self, num: uint) -> (uint, Option<&TclToken<'b>>) {
        if num == 0 {
            return (0, Some(self));
        }
        let mut numleft = num - 1;
        for subtok in self.tokens.iter() {
            match subtok.traverse(numleft) {
                (0, Some(tok)) => { return (0, Some(tok)); },
                (n, None) => { numleft = n; },
                _ => assert!(false),
            }
        }
        return (numleft, None);
    }
}
pub struct TclTokenIter<'a, 'b: 'a> {
    token: &'a TclToken<'b>,
    cur: uint,
}
impl<'b, 'c: 'b> Iterator<&'b TclToken<'c>> for TclTokenIter<'b, 'c> {
    fn next(&mut self) -> Option<&'b TclToken<'c>> {
        self.cur += 1;
        let ret: Option<&'b TclToken<'c>> = match self.token.traverse(self.cur-1) {
            (0, Some(tok)) => Some(tok),
            (0, None) => None,
            x => panic!("Invalid traverse return {}, iterator called after finish?", x),
        };
        return ret;
    }
}

/// Takes: a script
/// Returns:
/// - the comment prefixing the first command
/// - the first command
/// - the top level string tokens of the first command
/// - and the remaining script.
///
/// ```
/// use tclscan::rstcl::{TclParse,TclToken};
/// use tclscan::rstcl::TokenType::{SimpleWord,Word,Variable,Text,Command};
/// use tclscan::rstcl::parse_command;
/// assert!(parse_command("a b $c [d]") == (TclParse {
///     comment: "",
///     command: "a b $c [d]",
///     tokens: vec![
///         TclToken {
///             ttype: SimpleWord, val: "a",
///             tokens: vec![TclToken { ttype: Text, val: "a", tokens: vec![] }]
///         },
///         TclToken {
///             ttype: SimpleWord, val: "b",
///             tokens: vec![TclToken { ttype: Text, val: "b", tokens: vec![] }]
///         },
///         TclToken {
///             ttype: Word, val: "$c",
///             tokens: vec![
///                 TclToken {
///                     ttype: Variable, val: "$c",
///                     tokens: vec![TclToken { ttype: Text, val: "c", tokens: vec![] }]
///                 }
///             ]
///         },
///         TclToken {
///             ttype: Word, val: "[d]",
///             tokens: vec![TclToken { ttype: Command, val: "[d]", tokens: vec![] }]
///         }
///     ]
/// }, ""));
/// assert!(parse_command(" a\n") == (TclParse {
///     comment: "", command: "a\n",
///     tokens: vec![
///         TclToken {
///             ttype: SimpleWord, val: "a",
///             tokens: vec![TclToken { ttype: Text, val: "a", tokens: vec![] }]
///         }
///     ]
/// }, ""));
/// assert!(parse_command("a; b") == (TclParse {
///     comment: "", command: "a;",
///     tokens: vec![
///         TclToken {
///             ttype: SimpleWord, val: "a",
///             tokens: vec![TclToken { ttype: Text, val: "a", tokens: vec![] }]
///         }
///     ]
/// }, " b"));
/// assert!(parse_command("#comment\n\n\na\n") == (TclParse {
///     comment: "#comment\n", command: "a\n",
///     tokens: vec![
///         TclToken {
///             ttype: SimpleWord, val: "a",
///             tokens: vec![TclToken { ttype: Text, val: "a", tokens: vec![] }]
///         }
///     ]
/// }, ""));
/// ```
pub fn parse_command<'a>(script: &'a str) -> (TclParse<'a>, &'a str) {
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
            return (TclParse { comment: "", command: "", tokens: vec![] }, "");
        }
        let tclparse = make_tclparse(script, script_start, &parse);

        let command_len = parse.commandSize.to_uint().unwrap();
        let command_off = parse.commandStart as uint - script_start;
        let remaining = script[command_off+command_len..];

        tcl::Tcl_FreeParse(parse_ptr);
        return (tclparse, remaining);
    }
}

unsafe fn make_tclparse<'a>(script: &'a str, script_start: uint, tcl_parse: &tcl::Tcl_Parse) -> TclParse<'a> {

    // commentStart seems to be undefined if commentSize == 0
    let comment = match tcl_parse.commentSize.to_uint().unwrap() {
        0 => "",
        l => {
            let offset = tcl_parse.commentStart as uint - script_start;
            script[offset..offset+l]
        },
    };
    let command_len = tcl_parse.commandSize.to_uint().unwrap();
    let command_off = tcl_parse.commandStart as uint - script_start;
    let command = script[command_off..command_off+command_len];

    let mut acc = vec![];
    for i in range(0, tcl_parse.numTokens as int).rev() {
        let tcl_token = *(tcl_parse.tokenPtr).offset(i);
        assert!(tcl_token.start as uint > 0);
        let offset = tcl_token.start as uint - script_start;
        let token_size = tcl_token.size.to_uint().unwrap();
        let tokenval = script[offset..offset+token_size];
        make_tcltoken(&tcl_token, tokenval, &mut acc);
    }
    assert!(acc.len() == tcl_parse.numWords.to_uint().unwrap());
    acc.reverse();
    return TclParse { comment: comment, command: command, tokens: acc };
}

fn count_tokens(token: &TclToken) -> uint {
    token.tokens.iter().map(|t| count_tokens(t)).sum() + 1
}

fn make_tcltoken<'a>(tcl_token: &tcl::Tcl_Token, tokenval: &'a str, acc: &mut Vec<TclToken<'a>>) {
    let token_type: TokenType = FromPrimitive::from_uint(tcl_token._type.to_uint().unwrap()).unwrap();
    let num_subtokens = tcl_token.numComponents.to_uint().unwrap();

    let subtokens = match token_type {
        Word | ExpandWord => {
            let mut subtokens = vec![];
            let mut count = 0;
            while count < num_subtokens {
                assert!(acc.len() > 0);
                let tok = acc.pop().unwrap();
                count += count_tokens(&tok);
                subtokens.push(tok);
            }
            assert!(count == num_subtokens);
            subtokens
        },
        SimpleWord => {
            assert!(acc.len() > 0);
            assert!(num_subtokens == 1);
            let tok = acc.pop().unwrap();
            assert!(tok.ttype == Text);
            vec![tok]
        },
        Text | Bs => {
            assert!(num_subtokens == 0);
            vec![]
        },
        Command => {
            assert!(tokenval.char_at(0) == '[');
            assert!(num_subtokens == 0);
            vec![]
        },
        Variable => {
            assert!(acc.len() > 0);
            let tok = acc.pop().unwrap();
            assert!(tok.ttype == Text);
            let mut subtokens = vec![tok];
            let mut count = 1;
            while count < num_subtokens {
                assert!(acc.len() > 0);
                let tok = acc.pop().unwrap();
                count += match tok.ttype {
                    Text | Bs | Command | Variable => count_tokens(&tok),
                    _ => panic!("Invalid token type {}", tok.ttype),
                };
                subtokens.push(tok);
            }
            assert!(count == num_subtokens);
            subtokens
        },
        //SubExpr => ,
        //Operator => ,
        _ => panic!("Unrecognised token type {}", token_type),
    };
    acc.push(TclToken { val: tokenval, tokens: subtokens, ttype: token_type })
}
