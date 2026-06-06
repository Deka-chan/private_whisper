/// Normalize raw ASR output before insertion. Currently trims edges; kept as a
/// seam for future punctuation/casing tweaks.
pub fn normalize(raw: &str) -> String {
    raw.trim().to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn trims_surrounding_whitespace() {
        assert_eq!(normalize("  привет мир \n"), "привет мир");
    }

    #[test]
    fn empty_or_whitespace_becomes_empty() {
        assert_eq!(normalize("   \n\t"), "");
    }
}
