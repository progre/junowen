pub fn to_lang_source(tag: &str) -> Option<&'static str> {
    match tag {
        "ja" => Some(include_str!("ja.toml")),
        _ => None,
    }
}
