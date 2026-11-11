use std::sync::{
    OnceLock,
    atomic::{AtomicBool, Ordering},
};
static ENGLISH: AtomicBool = AtomicBool::new(false);
static MESSAGES: OnceLock<std::collections::HashMap<String, String>> = OnceLock::new();
pub fn set_language(language: &str) {
    ENGLISH.store(language == "en", Ordering::Relaxed);
}
pub fn text(value: &str) -> String {
    if ENGLISH.load(Ordering::Relaxed) {
        MESSAGES
            .get_or_init(|| {
                serde_json::from_str(include_str!("../assets/en.json"))
                    .expect("bundled translations")
            })
            .get(value)
            .cloned()
            .unwrap_or_else(|| value.into())
    } else {
        value.into()
    }
}

#[cfg(feature = "desktop")]
pub fn compare_titles(left: &str, right: &str, language: &str) -> std::cmp::Ordering {
    use icu_collator::CollatorBorrowed;
    static COLLATION: OnceLock<[CollatorBorrowed<'static>; 2]> = OnceLock::new();
    let collators = COLLATION.get_or_init(|| {
        ["zh-TW", "en"].map(|tag| {
            let locale: icu_locale_core::Locale = tag.parse().expect("bundled locale");
            CollatorBorrowed::try_new(locale.into(), Default::default()).expect("bundled collation")
        })
    });
    collators[usize::from(language == "en")].compare(left, right)
}
#[cfg(all(test, feature = "desktop"))]
mod tests {
    #[test]
    fn titles_follow_browser_locale_order() {
        for (language, expected) in [
            ("zh-TW", ["二", "三", "a", "B"]),
            ("en", ["a", "B", "三", "二"]),
        ] {
            let mut titles = ["三", "二", "B", "a"];
            titles.sort_by(|a, b| super::compare_titles(a, b, language));
            assert_eq!(titles, expected);
        }
    }
}
