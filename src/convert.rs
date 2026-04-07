pub fn slugify(text: &str) -> String {
    let s: String = text
        .to_lowercase()
        .trim()
        .chars()
        .filter(|c| {
            c.is_alphanumeric()
                || *c == '-'
                || ('\u{4e00}'..='\u{9fff}').contains(c)
                || c.is_whitespace()
        })
        .map(|c| if c.is_whitespace() { '-' } else { c })
        .collect::<String>();
    let s = s.trim_end_matches('-');
    if s.is_empty() {
        return "article".into();
    }
    let max_bytes = 200;
    let mut result = String::new();
    for c in s.chars() {
        if result.len() + c.len_utf8() > max_bytes {
            break;
        }
        result.push(c);
    }
    let result = result.trim_end_matches('-');
    if result.is_empty() {
        return "article".into();
    }
    result.to_string()
}

pub fn dedent_text(text: &str) -> String {
    let lines: Vec<&str> = text.lines().collect();
    if lines.is_empty() {
        return String::new();
    }
    let min_indent = lines
        .iter()
        .filter(|l| !l.trim().is_empty())
        .map(|l| l.chars().take_while(|c| *c == ' ' || *c == '\t').count())
        .min()
        .unwrap_or(0);
    if min_indent == 0 {
        return text.to_string();
    }
    lines
        .iter()
        .map(|l| {
            if l.trim().is_empty() {
                String::new()
            } else {
                l.get(min_indent..).unwrap_or(l).to_string()
            }
        })
        .collect::<Vec<_>>()
        .join("\n")
}

pub fn dedent_code_blocks(md: &str) -> String {
    let mut result = String::with_capacity(md.len());
    let mut pos = 0;
    while pos < md.len() {
        let rest = &md[pos..];
        if rest.starts_with("```") {
            let fence_end = rest[3..].find('\n').map(|i| i + 3).unwrap_or(rest.len());
            let fence_line = &rest[..fence_end];
            let content_start = pos + fence_end + 1;
            let close = md[content_start..].find("\n```");
            if let Some(ci) = close {
                let code = &md[content_start..content_start + ci];
                let dedented = dedent_text(code);
                result.push_str(fence_line);
                result.push('\n');
                result.push_str(&dedented);
                result.push_str("\n```\n");
                pos = content_start + ci + 4;
            } else {
                let code = &md[content_start..];
                let dedented = dedent_text(code);
                result.push_str(fence_line);
                result.push('\n');
                result.push_str(&dedented);
                break;
            }
        } else {
            let skip = rest
                .char_indices()
                .nth(1)
                .map(|(i, _)| i)
                .unwrap_or(rest.len());
            let tail = &rest[skip..];
            let next_fence = tail.find("```").map(|i| skip + i).unwrap_or(rest.len());
            result.push_str(&md[pos..pos + next_fence]);
            pos += next_fence;
        }
        while pos < md.len() && md[pos..].starts_with('\n') {
            pos += 1;
        }
    }
    result
}

pub fn html_to_md(html: &str) -> String {
    let raw = html2md::parse_html(html);
    let trimmed = raw.trim();
    dedent_code_blocks(trimmed)
}

static RE_COVER: std::sync::LazyLock<regex::Regex> = std::sync::LazyLock::new(|| {
    regex::Regex::new(r#"<p><img\s+alt="cover_image"[^>]*></p>"#).unwrap()
});
static RE_H1: std::sync::LazyLock<regex::Regex> =
    std::sync::LazyLock::new(|| regex::Regex::new(r"<h1[^>]*>.*?</h1>").unwrap());
static RE_AUTHOR: std::sync::LazyLock<regex::Regex> =
    std::sync::LazyLock::new(|| regex::Regex::new(r"<p>\s*原创.*?</p>").unwrap());

pub fn strip_content_html_header(html: &str) -> String {
    let s = RE_COVER.replace(html, "");
    let s = RE_H1.replace(&s, "");
    let s = RE_AUTHOR.replace(&s, "");
    s.trim().to_string()
}

pub fn clean_tail(text: &str) -> String {
    let mut t = text.to_string();
    for p in &[
        "\n预览时标签不可点",
        "\n微信扫一扫",
        "\n使用小程序",
        "\n打开小程序",
    ] {
        if let Some(i) = t.find(p) {
            t.truncate(i);
        }
    }
    for word in &[
        "知道了",
        "打开小程序",
        "打开",
        "允许",
        "取消",
        "视频",
        "小程序",
        "赞",
        "在看",
        "分享",
        "留言",
        "收藏",
        "听过",
        "跳过",
        "轻触阅读原文",
        "向上滑动看下一个",
        "向下滑动看下一个",
    ] {
        t = t.replace(&format!("\n{}", word), "");
    }
    t.trim().to_string()
}

pub fn article_to_md(
    title: &str,
    author: &str,
    mp_id: &str,
    url: &str,
    pic_url: &str,
    description: &str,
    ts: i64,
    html: &str,
) -> String {
    let q = |s: &str| -> String {
        let escaped = s.replace('\\', "\\\\").replace('"', "\\\"");
        format!("\"{}\"", escaped)
    };
    let mut fm = vec![
        "---".to_string(),
        format!("title: {}", q(title)),
        format!("author: {}", q(author)),
    ];
    if !pic_url.is_empty() {
        fm.push(format!("coverImage: {}", q(pic_url)));
    }
    if !url.is_empty() {
        fm.push(format!("url: {}", q(url)));
    }
    fm.push(format!("mp_id: {}", q(mp_id)));
    if !description.is_empty() {
        fm.push(format!("description: {}", q(description)));
    }
    if ts > 0 {
        if let Some(dt) = chrono::DateTime::from_timestamp(ts, 0) {
            fm.push(format!("publish_time: {}", dt.format("%Y-%m-%dT%H:%M:%S")));
        }
    }
    fm.push("---".into());
    let body = if html.is_empty() {
        String::new()
    } else {
        let md = html_to_md(html);
        clean_tail(&md)
    };
    format!("{}\n{}\n", fm.join("\n"), body)
}
