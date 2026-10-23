use std::{collections::BTreeMap, sync::OnceLock};
static MESSAGES: OnceLock<BTreeMap<String, String>> = OnceLock::new();
pub fn translate(language: &str, text: &str) -> String {
    if language != "en" {
        return text.into();
    }
    let messages = MESSAGES.get_or_init(|| {
        serde_json::from_str(include_str!("../../src/locales/messages.en.json"))
            .expect("validated translation catalogue")
    });
    if let Some(value) = messages.get(text) {
        return value.clone();
    }
    let mut entries: Vec<_> = messages.iter().collect();
    entries.sort_by_key(|(key, _)| std::cmp::Reverse(key.len()));
    let mut result = text.to_string();
    for (key, value) in entries {
        result = result.replace(key, value);
    }
    result
}
