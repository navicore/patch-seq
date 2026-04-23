//! Small string utilities used by the REPL input layer.

/// Find the largest byte index <= pos that is a valid char boundary.
/// This is a stable implementation of the nightly `str::floor_char_boundary`.
pub(crate) fn floor_char_boundary(s: &str, pos: usize) -> usize {
    if pos >= s.len() {
        s.len()
    } else if s.is_char_boundary(pos) {
        pos
    } else {
        // Walk backwards to find a valid boundary
        let mut p = pos;
        while p > 0 && !s.is_char_boundary(p) {
            p -= 1;
        }
        p
    }
}
