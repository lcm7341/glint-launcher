pub fn steam_key_values(text: &str) -> impl Iterator<Item = (String, String)> + '_ {
    text.lines().filter_map(key_value)
}

pub fn steam_value(text: &str, key: &str) -> Option<String> {
    steam_key_values(text)
        .find(|(found, _)| found == key)
        .map(|(_, value)| value)
}

fn key_value(line: &str) -> Option<(String, String)> {
    let (key, rest) = quoted(line.trim_start())?;
    let (value, _) = quoted(rest.trim_start())?;
    Some((key, value))
}

fn quoted(text: &str) -> Option<(String, &str)> {
    let mut letters = text.strip_prefix('"')?.char_indices();
    let mut value = String::new();
    while let Some((index, letter)) = letters.next() {
        match letter {
            '"' => return Some((value, &text[index + 2..])),
            '\\' => value.push(letters.next()?.1),
            _ => value.push(letter),
        }
    }
    None
}
