//! Small string utilities used by the REPL input layer.

/// Find the largest byte index <= pos that is a valid char boundary.
///
/// Polyfill for the nightly `str::floor_char_boundary`; delete this and
/// switch call sites once that API stabilizes.
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_string_any_pos_yields_zero() {
        assert_eq!(floor_char_boundary("", 0), 0);
        assert_eq!(floor_char_boundary("", 10), 0);
    }

    #[test]
    fn ascii_positions_are_all_boundaries() {
        let s = "hello";
        for p in 0..=s.len() {
            assert_eq!(floor_char_boundary(s, p), p);
        }
    }

    #[test]
    fn pos_past_end_clamps_to_length() {
        let s = "hi";
        assert_eq!(floor_char_boundary(s, 5), s.len());
    }

    #[test]
    fn mid_multibyte_walks_back_to_start_of_char() {
        // "é" is 2 bytes (0xC3 0xA9), so byte indices 0 and 2 are boundaries but 1 is not.
        let s = "é";
        assert_eq!(s.len(), 2);
        assert_eq!(floor_char_boundary(s, 0), 0);
        assert_eq!(floor_char_boundary(s, 1), 0);
        assert_eq!(floor_char_boundary(s, 2), 2);
    }

    #[test]
    fn mid_emoji_walks_back_to_start() {
        // "😀" is 4 bytes; only 0 and 4 are boundaries.
        let s = "😀";
        assert_eq!(s.len(), 4);
        assert_eq!(floor_char_boundary(s, 1), 0);
        assert_eq!(floor_char_boundary(s, 2), 0);
        assert_eq!(floor_char_boundary(s, 3), 0);
        assert_eq!(floor_char_boundary(s, 4), 4);
    }
}
