//! Configuration options for the match-sorting algorithm.
//!
//! [`MatchSorterOptions`] controls global behavior such as diacritics handling
//! that applies across all keys during match evaluation.

/// Global options that control match-sorting behavior.
///
/// # Defaults
///
/// All fields default to their most common usage:
/// - `keep_diacritics`: `false` (diacritics are stripped before comparison)
///
/// # Examples
///
/// ```
/// use matchsorter::MatchSorterOptions;
///
/// // Default options: strip diacritics
/// let opts = MatchSorterOptions::default();
/// assert!(!opts.keep_diacritics);
///
/// // Preserve diacritics
/// let opts = MatchSorterOptions { keep_diacritics: true, ..Default::default() };
/// assert!(opts.keep_diacritics);
/// ```
#[derive(Debug, Clone, Default, PartialEq)]
pub struct MatchSorterOptions {
    /// When `true`, diacritics (accents, combining marks) are preserved during
    /// comparison. When `false` (default), diacritics are stripped so that
    /// e.g. "cafe" matches "caf\u{00e9}".
    pub keep_diacritics: bool,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_keep_diacritics_is_false() {
        let opts = MatchSorterOptions::default();
        assert!(!opts.keep_diacritics);
    }

    #[test]
    fn debug_formatting() {
        let opts = MatchSorterOptions {
            keep_diacritics: true,
        };
        let debug_str = format!("{opts:?}");
        assert!(debug_str.contains("keep_diacritics"));
        assert!(debug_str.contains("true"));
    }

    #[test]
    fn clone_produces_equal_value() {
        let opts = MatchSorterOptions {
            keep_diacritics: true,
        };
        let cloned = opts.clone();
        assert_eq!(cloned.keep_diacritics, opts.keep_diacritics);
    }

    #[test]
    fn struct_update_syntax() {
        let opts = MatchSorterOptions {
            keep_diacritics: true,
        };
        assert!(opts.keep_diacritics);
    }
}
