// use std::collections::HashMap;

// use crate::tape::Tape;

// struct BuiltinMacros;

// impl<'a> BuiltinMacros {
// $ keys like $file get sent to compiler/editor, not DOM
// {alex,pg-1}

// in LSP, add/remove table rows in sync
// up, down, left, right
// hr spreads
// vr via |

// in editor, show cell coords in gutter

/*
If I type 12,150.00 in the \csv block and hit save, and your Rust formatter
instantly aligns it and calculates the total, that "magic" is addictive.
*/
// sum is fn taking one arg

// like intellij,
//  use hints to align columns
//  fold/render equations, templates

// sum|add avg|mean count min/max abs/round 
// If your language can't handle SUM(C2:C5, C7:C8), the user is forced to write C2+C3+C4+C5+C7+C8
#[total-col,stripe]
\table[
    c = total            // shorthand          
    c2..c4 = sum(a_..b_)      // infer from difference
]{
    Quantity
    ==
    12
    --
    45
    128
    7
}{
    Unit Price
    150.00
    24.50
    3.15
    1240.99
}

// visual editor
// wysiwyg editor

// formatter aligns csv values by column
    //table.style = striped // or...

// make schema for supported operations on identifiers

\table(csv,stripe,transpose,col-head)     // stripe-col total|total-row stripe|stripe-row
[
    $name = income
    col-2.{
        unit = usd      // inherit unit; these are strings
        align = right
    }
    row-2.div-below                 // div|div-below, div-above, div-r|div-right, div-l|div-left
    col-3.{
        fill = col-1*col-2
        div-l
    }
    row-6.span = {1,2}          // merge between both columns and rows by going thru 4 all dimensions--leaving a dot which is then pruned

]{
    Quantity, Unit Price, Total
    12,       150
    2,        3
    1,        2
    15,       4

    {table.other}
}

//     // highlight
//     // \mark.1{} \mark.2{} \mark.3{} \mark.4{} \mark.5{} \mark.6{}
//     fn mark_n() {

//     }

//     // \if[<expr>]{} \if[<expr>]{}{}
//     // if `name` is set
//     fn _if(cond: , body: &'a [u8]) {

//     }

\style[color=red]{

}

//     // \style[<css>]{<>}

//     // \eval{sum(A2:A5)}
// verrry complex
//     // fn eval(state , body: &'a [u8]) {

//     // }

//     // \table[<opts>]{}{}{..
//     // in opt, define pgraph_spacing
//     // empty lines? use ---
//     fn table() {

//     }

//     // center justify right-align
//     // \begin[<macro>] \end[<macro>]

//     // \column{}{}{..

//     // \grid{}{}{..    // auto-optimize for incomplete rows

//     //
//     fn _use() {

//     }

// }
