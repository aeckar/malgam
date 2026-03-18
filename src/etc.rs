/// Counts the number of tabs or the number of space characters divided by 4 (floored).
/// 
/// Used to determine separation between table cells and indentation of list items.
/// For optimal performance, the given string should only consist of whitespace characters.
pub fn count_indent(ws: &[u8]) -> u8 {
    let mut tabs = 0;
    let mut spaces = 0;
    for &ch in ws {
        if ch == b' ' {
            spaces += 1;
        } else if ch == b'\t' {
            tabs += 1;
        }
    }
    tabs + (spaces / 4)
}