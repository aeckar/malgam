use std::collections::HashMap;

use crate::tape::Tape;

pub enum MacroArg<'a> {
    Bool(bool),
    String(&'a [u8]),
    Number(f64),
}

impl<'a> MacroArg<'a> {
    /// Returns `none` if the argument list is malformed.
    fn parse_all(args: &'a [u8]) -> Option<HashMap<&'a [u8], MacroArg<'a>>> {
        let mut args = Tape::new(args);
        let values = HashMap::new();
        while let Some(&ch) = args.next() {
            let key = args.consume(|ch,_| ch.is_ascii_alphabetic());
            if key.is_empty() {
                
            }
        }
        values
    }
}

struct BuiltinMacros;

impl<'a> BuiltinMacros {
    // highlight
    // !1{} \2{} \3{} \4{} \5{} \6{}
    fn h1() {

    }

    // \if[<expr>]{}
    // if `name` is set
    fn _if(cond: , body: &'a [u8]) {

    }

    // builtin globals
    // \file[<opts>]
    // code, finance, use ascii-math?
    fn file() {

    }

    // \style{<css>}{<>}

    // \eval{<expr>}
    // VERYYYYYYY COMPLEX
    // fn eval(state , body: &'a [u8]) {
        
    // }

    // \table[<opts>]{}{}{..
    // in opt, define pgraph_spacing
    // empty lines? use ---
    fn table() {

    }

    // center justify right-align
    // \begin[<macro>] \end[<macro>]

    // \col{}{}{..

    // \grid{}{}{..    // auto-optimize for incomplete rows

    // ''line quote
    // '''
    // block quote
    // '''

    // 
    fn _use() {

    }

}
