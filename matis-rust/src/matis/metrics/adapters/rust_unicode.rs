//! Strict Unicode 16 name/alias lookup from the parent's pinned unicode_names2 =1.3.0.
//! Python's database depends on its interpreter version; local Python 3.11 uses Unicode 14.
//! Algorithmic CJK/Hangul names are case-sensitive in Python, unlike ordinary names/aliases.
/// Look up a Unicode 16 character name or alias, returning `None` for invalid spellings.
/// Enforce Python-style case sensitivity for algorithmic Hangul and CJK names;
/// ordinary names and aliases use the pinned Unicode database's lookup rules.
pub fn character(name: &str) -> Option<char> {
    let value = unicode_names2::character(name)?;
    let canonical = unicode_names2::name(value).map(|name| name.to_string()).unwrap_or_default();
    if canonical.starts_with("HANGUL SYLLABLE ") {
        return (name == canonical).then_some(value);
    }
    if canonical.starts_with("CJK UNIFIED IDEOGRAPH-") && name != name.to_ascii_uppercase() {
        return None;
    }
    Some(value)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn names_and_aliases_preserve_strict_spelling() {
        assert_eq!(character("latin small letter a"), Some('a'));
        assert_eq!(character("backspace"), Some('\u{8}'));
        assert_eq!(character("BYTE ORDER MARK"), Some('\u{feff}'));
        assert_eq!(character("VS256"), Some('\u{e01ef}'));
        assert_eq!(character("Black_Star"), None);
        assert_eq!(character("latinsmalllettera"), None);
        assert_eq!(character("NEW_LINE"), None);
        assert_eq!(character("VS0256"), None);
    }
    #[test]
    fn algorithmic_names_keep_python_case_rules_and_the_selected_unicode16_data() {
        assert_eq!(character("CJK UNIFIED IDEOGRAPH-04E00"), Some('一'));
        assert_eq!(character("cjk unified ideograph-4e00"), None);
        assert_eq!(character("HANGUL SYLLABLE GA"), Some('가'));
        assert_eq!(character("hangul syllable ga"), None);
        assert_eq!(character("KAWI SIGN REPHA"), Some('\u{11f02}'));
    }
}
