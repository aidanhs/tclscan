use std::mem::uninitialized;
use std::iter::AdditiveIterator;
use std::num::FromPrimitive;
use std::ffi::CString;

use tcl;
use self::TokenType::*;

static mut I: Option<*mut tcl::Tcl_Interp> = None;
unsafe fn tcl_interp() -> *mut tcl::Tcl_Interp {
    if I.is_none() {
        I = Some(tcl::Tcl_CreateInterp());
    }
    return I.unwrap();
}

#[derive(Copy, Debug, FromPrimitive, PartialEq)]
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

#[derive(Debug, PartialEq)]
pub struct TclParse<'a> {
    pub comment: Option<&'a str>,
    pub command: Option<&'a str>,
    pub tokens: Vec<TclToken<'a>>,
}
#[derive(Debug, PartialEq)]
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
    fn traverse(&self, num: usize) -> (usize, Option<&TclToken<'b>>) {
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
    cur: usize,
}
impl<'b, 'c: 'b> Iterator for TclTokenIter<'b, 'c> {
    type Item = &'b TclToken<'c>;
    fn next(&mut self) -> Option<&'b TclToken<'c>> {
        self.cur += 1;
        let ret: Option<&'b TclToken<'c>> = match self.token.traverse(self.cur-1) {
            (0, Some(tok)) => Some(tok),
            (0, None) => None,
            x => panic!("Invalid traverse return {:?}, iterator called after finish?", x),
        };
        return ret;
    }
}

/// Takes: a string, which should be a tcl script
/// Returns: a parse structure and the remaining string.
///
/// ```
/// use tclscan::rstcl::{TclParse,TclToken};
/// use tclscan::rstcl::TokenType::{SimpleWord,Word,Variable,Text,Command};
/// use tclscan::rstcl::parse_command;
/// assert!(parse_command("a b $c [d]") == (TclParse {
///     comment: Some(""), command: Some("a b $c [d]"),
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
///     comment: Some(""), command: Some("a\n"),
///     tokens: vec![
///         TclToken {
///             ttype: SimpleWord, val: "a",
///             tokens: vec![TclToken { ttype: Text, val: "a", tokens: vec![] }]
///         }
///     ]
/// }, ""));
/// assert!(parse_command("a; b") == (TclParse {
///     comment: Some(""), command: Some("a;"),
///     tokens: vec![
///         TclToken {
///             ttype: SimpleWord, val: "a",
///             tokens: vec![TclToken { ttype: Text, val: "a", tokens: vec![] }]
///         }
///     ]
/// }, " b"));
/// assert!(parse_command("#comment\n\n\na\n") == (TclParse {
///     comment: Some("#comment\n"), command: Some("a\n"),
///     tokens: vec![
///         TclToken {
///             ttype: SimpleWord, val: "a",
///             tokens: vec![TclToken { ttype: Text, val: "a", tokens: vec![] }]
///         }
///     ]
/// }, ""));
/// ```
pub fn parse_command<'a>(string: &'a str) -> (TclParse<'a>, &'a str) {
    return parse(string, true, false);
}
/// Takes: a TokenType::Command token contained in '[]'
/// Returns: a parse structure
pub fn parse_command_token<'a>(token: &'a TclToken) -> TclParse<'a> {
    assert!(token.ttype == TokenType::Command);
    assert!(token.val.starts_with("[") && token.val.ends_with("]"));
    let cmd = &token.val[1..token.val.len()-1];
    let (parse, remaining) = parse_command(cmd);
    assert!(remaining.trim() == "");
    return parse;
}
/// Takes: a string, which should be a tcl expr
/// Returns: a parse structure and the remaining script.
///
/// ```
/// use tclscan::rstcl::{TclParse,TclToken};
/// use tclscan::rstcl::TokenType::{SubExpr,Text,Variable,Command,Operator};
/// use tclscan::rstcl::parse_expr;
/// assert!(parse_expr("[a]+$b+cos([c]+$d)") == (TclParse {
///     comment: None, command: None,
///     tokens: vec![
///         TclToken {
///             ttype: SubExpr, val: "[a]+$b+cos([c]+$d)",
///             tokens: vec![
///                 TclToken { ttype: Operator, val: "+", tokens: vec![] },
///                 TclToken {
///                     ttype: SubExpr, val: "[a]+$b",
///                     tokens: vec![
///                         TclToken { ttype: Operator, val: "+", tokens: vec![] },
///                         TclToken {
///                             ttype: SubExpr, val: "[a]",
///                             tokens: vec![
///                                 TclToken { ttype: Command, val: "[a]", tokens: vec![] }
///                             ]
///                         },
///                         TclToken {
///                             ttype: SubExpr, val: "$b",
///                             tokens: vec![
///                                 TclToken {
///                                     ttype: Variable, val: "$b",
///                                     tokens: vec![
///                                         TclToken { ttype: Text, val: "b", tokens: vec![] }
///                                     ]
///                                 }
///                             ]
///                         }
///                     ]
///                 },
///                 TclToken {
///                     ttype: SubExpr, val: "cos([c]+$d)",
///                     tokens: vec![
///                         TclToken { ttype: Operator, val: "cos", tokens: vec![] },
///                         TclToken {
///                             ttype: SubExpr, val: "[c]+$d",
///                             tokens: vec![
///                                 TclToken { ttype: Operator, val: "+", tokens: vec![] },
///                                 TclToken {
///                                     ttype: SubExpr, val: "[c]",
///                                     tokens: vec![
///                                         TclToken { ttype: Command, val: "[c]", tokens: vec![] }
///                                     ]
///                                 },
///                                 TclToken {
///                                     ttype: SubExpr, val: "$d",
///                                     tokens: vec![
///                                         TclToken {
///                                             ttype: Variable, val: "$d",
///                                             tokens: vec![
///                                                 TclToken { ttype: Text, val: "d", tokens: vec![] }
///                                             ]
///                                         }
///                                     ]
///                                 }
///                             ]
///                         }
///                     ]
///                 }
///             ]
///         }
///     ]
/// }, ""));
/// assert!(parse_expr("1") == (TclParse {
///     comment: None, command: None,
///     tokens: vec![
///         TclToken {
///             ttype: SubExpr, val: "1",
///             tokens: vec![TclToken { ttype: Text, val: "1", tokens: vec![] }]
///         }
///     ]
/// }, ""));
/// ```
pub fn parse_expr<'a>(string: &'a str) -> (TclParse<'a>, &'a str) {
    return parse(string, false, true);
}

fn parse<'a>(string: &'a str, is_command: bool, is_expr: bool) -> (TclParse<'a>, &'a str) {
    unsafe {
        let mut parse: tcl::Tcl_Parse = uninitialized();
        let parse_ptr: *mut tcl::Tcl_Parse = &mut parse;

        // https://github.com/rust-lang/rust/issues/16035
        let string_cstr = CString::from_slice(string.as_bytes());
        let string_ptr = string_cstr.as_ptr();
        let string_start = string_ptr as usize;

        let parsed = match (is_command, is_expr) {
            // interp, start, numBytes, nested, parsePtr
            (true, false) => tcl::Tcl_ParseCommand(tcl_interp(), string_ptr, -1, 0, parse_ptr),
            // interp, start, numBytes, parsePtr
            (false, true) => tcl::Tcl_ParseExpr(tcl_interp(), string_ptr, -1, parse_ptr),
            parse_args => panic!("Don't know how to parse {:?}", parse_args),
        };
        if parsed != 0 {
            println!("WARN: couldn't parse {}", string);
            return (TclParse { comment: Some(""), command: Some(""), tokens: vec![] }, "");
        }
        let tokens = make_tokens(string, string_start, &parse);

        let (tclparse, remaining) = match (is_command, is_expr) {
            (true, false) => {
                assert!(tokens.len() == parse.numWords as usize);
                // commentStart seems to be undefined if commentSize == 0
                let comment = Some(match parse.commentSize as usize {
                    0 => "",
                    l => {
                        let offset = parse.commentStart as usize - string_start;
                        &string[offset..offset+l]
                    },
                });
                let command_len = parse.commandSize as usize;
                let command_off = parse.commandStart as usize - string_start;
                let command = Some(&string[command_off..command_off+command_len]);
                let remaining = &string[command_off+command_len..];
                (TclParse { comment: comment, command: command, tokens: tokens }, remaining)
            },
            (false, true) => {
                (TclParse { comment: None, command: None, tokens: tokens }, "")
            },
            _ => panic!("Unreachable"),
        };

        tcl::Tcl_FreeParse(parse_ptr);
        return (tclparse, remaining);
    }
}

unsafe fn make_tokens<'a>(string: &'a str, string_start: usize, tcl_parse: &tcl::Tcl_Parse) -> Vec<TclToken<'a>> {
    let mut acc = vec![];
    for i in range(0, tcl_parse.numTokens as isize).rev() {
        let tcl_token = *(tcl_parse.tokenPtr).offset(i);
        assert!(tcl_token.start as usize > 0);
        let offset = tcl_token.start as usize - string_start;
        let token_size = tcl_token.size as usize;
        let tokenval = &string[offset..offset+token_size];
        make_tcltoken(&tcl_token, tokenval, &mut acc);
    }
    acc.reverse();
    return acc;
}

fn count_tokens(token: &TclToken) -> usize {
    token.tokens.iter().map(|t| count_tokens(t)).sum() + 1
}

fn make_tcltoken<'a>(tcl_token: &tcl::Tcl_Token, tokenval: &'a str, acc: &mut Vec<TclToken<'a>>) {
    let token_type: TokenType = FromPrimitive::from_uint(tcl_token._type as usize).unwrap();
    let num_subtokens = tcl_token.numComponents as usize;

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
                    _ => panic!("Invalid token type {:?}", tok.ttype),
                };
                subtokens.push(tok);
            }
            assert!(count == num_subtokens);
            subtokens
        },
        SubExpr => {
            assert!(acc.len() > 0);
            let start_ttype = acc[acc.len()-1].ttype;
            let mut subtokens = vec![];
            let mut count = 0;
            if start_ttype == Operator {
                subtokens.push(acc.pop().unwrap());
                count += 1;
            }
            while count < num_subtokens {
                assert!(acc.len() > 0);
                let tok = acc.pop().unwrap();
                if start_ttype == Operator {
                    assert!(tok.ttype == SubExpr);
                }
                match tok.ttype {
                    Word | Text | Bs | Command | Variable | SubExpr => {
                        count += count_tokens(&tok)
                    },
                    _ => panic!("Invalid token {:?}", tok.ttype),
                }
                subtokens.push(tok);
            }
            assert!(count == num_subtokens);
            subtokens
        },
        Operator => {
            assert!(acc.len() > 0);
            assert!(num_subtokens == 0);
            vec![]
        },
    };
    acc.push(TclToken { val: tokenval, tokens: subtokens, ttype: token_type })
}
