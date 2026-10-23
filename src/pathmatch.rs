//! The pure matching behind `system.fuzzy_match` and `system.path_compare`.
//!
//! Both work on byte strings the way `src/api/system.c` walked NUL-terminated `const char *`:
//! reads past the end give 0, and digit runs wrap like `size_t`. The Lua argument handling stays
//! in `system.rs`.

use crate::luaargs::nul_cut;

/// `f_fuzzy_match`: case-insensitive subsequence score of `p` in `s`, or `None` for no match.
/// `files` scores from the end of both strings, so a match in the file name weighs most.
pub(crate) fn fuzzy_score(s: &[u8], p: &[u8], files: bool) -> Option<i64> {
    let (str_len, ptn_len) = (s.len() as i64, p.len() as i64);

    // Indexing with `at`, which reads 0 outside the string like the C's terminator.
    let at = |buf: &[u8], i: i64| -> u8 { if i >= 0 && (i as usize) < buf.len() { buf[i as usize] } else { 0 } };
    let lower = |c: u8| c.to_ascii_lowercase();

    let (mut score, mut run) = (0i32, 0i32);
    let increment: i64 = if files { -1 } else { 1 };
    let mut st = if files { str_len - 1 } else { 0 };
    let mut pt = if files { ptn_len - 1 } else { 0 };

    while st >= 0 && pt >= 0 && at(&s, st) != 0 && at(&p, pt) != 0 {
        while st >= 0 && at(&s, st) == b' ' {
            st += increment;
        }
        while pt >= 0 && at(&p, pt) == b' ' {
            pt += increment;
        }
        if lower(at(&s, st)) == lower(at(&p, pt)) {
            score += run * 10 - (at(&s, st) != at(&p, pt)) as i32;
            run += 1;
            pt += increment;
        } else {
            score -= 10;
            run = 0;
        }
        st += increment;
    }
    if pt >= 0 && at(&p, pt) != 0 {
        return None;
    }
    Some((score - (str_len as i32) * 10) as i64)
}

/// `f_path_compare`: whether path `p1` (of type `t1`, "dir" or not) sorts before `p2`:
/// directories first, numeric runs as integers, `/` as a terminator.
pub(crate) fn path_before(p1: &[u8], t1: &[u8], p2: &[u8], t2: &[u8]) -> bool {
    let (len1, len2) = (p1.len(), p2.len());

    // Reads 0 past the end, like the C's terminator.
    let at = |b: &[u8], i: usize| -> u8 { b.get(i).copied().unwrap_or(0) };
    // `strcmp(type, "dir")` sees the type up to its first NUL.
    let mut type1 = (nul_cut(t1) != b"dir") as i32;
    let mut type2 = (nul_cut(t2) != b"dir") as i32;

    let mut offset = 0usize;
    let mut i = 0;
    while i < len1 && i < len2 {
        if p1[i] != p2[i] {
            break;
        }
        if p1[i] == b'/' {
            offset = i + 1;
        }
        i += 1;
    }
    // `strchr(path + offset, PATHSEP)` stops at the first NUL.
    let has_sep = |b: &[u8]| b.iter().skip(offset).take_while(|&&c| c != 0).any(|&c| c == b'/');
    if has_sep(&p1) {
        type1 = 0;
    }
    if has_sep(&p2) {
        type2 = 0;
    }
    if type1 != type2 {
        return type1 < type2;
    }

    let mut cfr: i32 = -1;
    let same_len = len1 == len2;
    let (mut i, mut j) = (offset, offset);
    while i <= len1 && j <= len2 {
        let (a, b) = (at(&p1, i), at(&p2, j));
        if a == 0 || b == 0 {
            if cfr < 0 {
                cfr = 0;
            }
            if !same_len {
                cfr = (a == 0) as i32;
            }
        } else if a.is_ascii_digit() && b.is_ascii_digit() {
            let (mut ii, mut ij) = (0usize, 0usize);
            while at(&p1, i + ii).is_ascii_digit() {
                ii += 1;
            }
            while at(&p2, j + ij).is_ascii_digit() {
                ij += 1;
            }
            // size_t arithmetic in the C: wraps on overflow.
            let (mut di, mut dj) = (0usize, 0usize);
            for k in 0..ii {
                di = di.wrapping_mul(10).wrapping_add((at(&p1, i + k) - b'0') as usize);
            }
            for k in 0..ij {
                dj = dj.wrapping_mul(10).wrapping_add((at(&p2, j + k) - b'0') as usize);
            }
            if di == dj {
                i += 1;
                j += 1;
                continue;
            }
            cfr = (di < dj) as i32;
        } else if a == b {
            i += 1;
            j += 1;
            continue;
        } else if a == b'/' || b == b'/' {
            cfr = (a == b'/') as i32;
        } else {
            let (la, lb) = (a.to_ascii_lowercase_c(), b.to_ascii_lowercase_c());
            if la == lb {
                if same_len && cfr < 0 {
                    cfr = (a > b) as i32;
                }
                i += 1;
                j += 1;
                continue;
            }
            cfr = (la < lb) as i32;
        }
        break;
    }
    cfr != 0
}

/// The C folds `'A'..='Z'` by adding 32 on a `char`; identical to this for ASCII.
trait AsciiLowerC {
    fn to_ascii_lowercase_c(self) -> u8;
}
impl AsciiLowerC for u8 {
    fn to_ascii_lowercase_c(self) -> u8 {
        if self.is_ascii_uppercase() { self + 32 } else { self }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fuzzy(s: &str, p: &str) -> Option<i64> {
        fuzzy_score(s.as_bytes(), p.as_bytes(), false)
    }

    fn files(s: &str, p: &str) -> Option<i64> {
        fuzzy_score(s.as_bytes(), p.as_bytes(), true)
    }

    fn before(p1: &str, t1: &str, p2: &str, t2: &str) -> bool {
        path_before(p1.as_bytes(), t1.as_bytes(), p2.as_bytes(), t2.as_bytes())
    }

    #[test]
    fn fuzzy_needs_every_pattern_byte_in_order() {
        assert_eq!(fuzzy("abc", "cba"), None);
        assert_eq!(fuzzy("ab", "abc"), None);
        assert!(fuzzy("xaxbxc", "abc").is_some());
        assert_eq!(fuzzy("abc", ""), Some(-30));
        assert_eq!(fuzzy("", ""), Some(0));
        assert_eq!(fuzzy("", "a"), None);
    }

    #[test]
    fn fuzzy_ignores_case_but_prefers_exact_case() {
        assert!(fuzzy("abc", "abc") > fuzzy("ABC", "abc"));
        assert_eq!(fuzzy("ABC", "abc"), fuzzy("abc", "ABC"));
    }

    #[test]
    fn fuzzy_rewards_runs_and_skips_spaces() {
        assert!(fuzzy("renderer", "rend") > fuzzy("r_e_n_d", "rend"));
        assert_eq!(fuzzy("a b c", "abc"), Some(-20 - 30 + 30));
        assert_eq!(fuzzy("abc", "a b c"), fuzzy("abc", "abc"));
    }

    #[test]
    fn fuzzy_files_mode_scans_from_the_end() {
        assert!(files("src/renderer.c", "renderer").is_some());
        assert_ne!(files("src/renderer.c", "c"), fuzzy("src/renderer.c", "c"));
    }

    #[test]
    fn fuzzy_stops_at_an_embedded_nul() {
        assert_eq!(fuzzy("a\0b", "ab"), None);
        assert_eq!(fuzzy("ab", "a\0b"), fuzzy("ab", "a"));
    }

    #[test]
    fn directories_sort_before_files() {
        assert!(before("b", "dir", "a", "file"));
        assert!(!before("a", "file", "b", "dir"));
    }

    #[test]
    fn a_path_below_the_common_prefix_counts_as_a_directory() {
        assert!(before("src/x", "file", "top", "file"));
        assert!(!before("top", "file", "src/x", "file"));
    }

    #[test]
    fn digit_runs_compare_as_numbers() {
        assert!(before("file2", "file", "file10", "file"));
        assert!(!before("file10", "file", "file2", "file"));
        assert!(before("a9b", "file", "a10", "file"));
    }

    #[test]
    fn letters_compare_ignoring_case() {
        assert!(before("apple", "file", "Banana", "file"));
        assert!(!before("Banana", "file", "apple", "file"));
    }

    #[test]
    fn equal_paths_do_not_sort_before_each_other() {
        assert!(!before("a", "file", "a", "file"));
    }

    #[test]
    fn a_type_is_read_up_to_its_nul() {
        assert!(before("b", "dir\0file", "a", "file"));
    }

    #[test]
    fn huge_digit_runs_wrap_like_size_t() {
        let big = "9".repeat(25);
        // Must not panic in a debug build.
        let _ = before(&big, "file", "1", "file");
    }
}
