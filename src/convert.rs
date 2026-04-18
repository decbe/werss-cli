use anyhow::anyhow;

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
        if let Some(stripped) = rest.strip_prefix("```") {
            let fence_end = stripped.find('\n').map(|i| i + 3).unwrap_or(rest.len());
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

pub fn convert_html(html: &str) -> ConversionResult {
    convert_html_internal(html, false)
}

pub fn convert_html_for_download(html: &str) -> ConversionResult {
    convert_html_internal(html, true)
}

fn convert_html_internal(html: &str, localize_image_refs: bool) -> ConversionResult {
    // 提取 HTML 中所有 img src 的外部 URL
    let img_urls: Vec<(String, String)> = extract_img_srcs(html);

    let result = html_to_markdown_rs::convert(html, None)
        .map_err(|e| anyhow!("HTML conversion failed: {}", e))
        .unwrap();
    let mut markdown = result.content.unwrap_or_default();

    let mut images = Vec::new();
    for (i, (url, ext)) in img_urls.iter().enumerate() {
        if localize_image_refs {
            let filename = format!("{}.{}", i, ext);
            let old_ref = format!("]({})", url);
            let new_ref = format!("](imgs/{})", filename);
            markdown = markdown.replace(&old_ref, &new_ref);
        }
        images.push(ExtractedImage {
            index: i,
            format: ext.clone(),
            url: url.clone(),
        });
    }

    let markdown = dedent_code_blocks(&markdown);
    ConversionResult { markdown, images }
}

/// 从 HTML 中提取所有 <img src="..."> 的 URL 和扩展名
fn extract_img_srcs(html: &str) -> Vec<(String, String)> {
    static RE_IMG: std::sync::LazyLock<regex::Regex> =
        std::sync::LazyLock::new(|| regex::Regex::new(r#"<img[^>]+src=["']([^"']+)["']"#).unwrap());
    let mut results = Vec::new();
    for cap in RE_IMG.captures_iter(html) {
        let url = cap
            .get(1)
            .map(|m| m.as_str().to_string())
            .unwrap_or_default();
        if url.is_empty() {
            continue;
        }
        let ext = image_ext_from_url(&url);
        results.push((url, ext));
    }
    results
}

pub fn image_ext_from_url(url: &str) -> String {
    let path = url.split('?').next().unwrap_or(url);
    let path_lower = path.to_lowercase();
    if path_lower.ends_with(".png") {
        "png".to_string()
    } else if path_lower.ends_with(".jpg") || path_lower.ends_with(".jpeg") {
        "jpg".to_string()
    } else if path_lower.ends_with(".gif") {
        "gif".to_string()
    } else if path_lower.ends_with(".webp") {
        "webp".to_string()
    } else if path_lower.ends_with(".bmp") {
        "bmp".to_string()
    } else {
        "png".to_string()
    }
}

pub struct ExtractedImage {
    pub index: usize,
    pub format: String,
    pub url: String,
}

pub struct ConversionResult {
    pub markdown: String,
    pub images: Vec<ExtractedImage>,
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

#[allow(clippy::too_many_arguments)]
pub fn article_to_md(
    title: &str,
    author: &str,
    mp_id: &str,
    url: &str,
    pic_url: &str,
    description: &str,
    ts: i64,
    html: &str,
    download_images: bool,
) -> (String, Vec<ExtractedImage>) {
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
    let (body, extracted_images) = if html.is_empty() {
        (String::new(), Vec::new())
    } else {
        let conv = if download_images {
            convert_html_for_download(html)
        } else {
            convert_html(html)
        };
        (clean_tail(&conv.markdown), conv.images)
    };
    (format!("{}\n{}\n", fm.join("\n"), body), extracted_images)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn convert_html_keeps_remote_image_urls_by_default() {
        let html = r#"<p>Hello</p><p><img src="https://example.com/demo.png"></p>"#;

        let result = convert_html(html);

        assert!(result.markdown.contains("https://example.com/demo.png"));
        assert!(!result.markdown.contains("imgs/0.png"));
        assert_eq!(result.images.len(), 1);
    }

    #[test]
    fn convert_html_rewrites_image_urls_when_localized() {
        let html = r#"<p>Hello</p><p><img src="https://example.com/demo.png"></p>"#;

        let result = convert_html_for_download(html);

        assert!(result.markdown.contains("imgs/0.png"));
        assert!(!result.markdown.contains("https://example.com/demo.png"));
        assert_eq!(result.images.len(), 1);
    }
}
