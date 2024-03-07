pub fn str_cap(s: &str) -> String {
    format!(
        "{}{}",
        s.chars().next().unwrap().to_uppercase(),
        s.chars().skip(1).collect::<String>()
    )
}
