use regex::Regex;

/// 在文本中搜索 flag{...} 格式，header 可自定义（默认 "flag"）。
// FIXME: 与前端 flagRegex 不一致——前端带 i 标志（可匹配 FLAG{...}），
// 这里没有 (?i)，auto_detect 的 flag 加权会漏掉大写头的 flag
pub fn find_flag(text: &str, header: &str) -> Option<String> {
    let h = normalize_header(header);
    let pattern = format!(r"{}\{{[^{{}}]+\}}", regex::escape(&h));
    let re = Regex::new(&pattern).ok()?;
    re.find(text).map(|m| m.as_str().to_string())
}

fn normalize_header(h: &str) -> String {
    let trimmed = h.trim();
    let base = trimmed.split('{').next().unwrap_or(trimmed);
    if base.is_empty() { "flag".to_string() } else { base.to_string() }
}
