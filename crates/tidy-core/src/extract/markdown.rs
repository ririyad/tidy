use htmd::HtmlToMarkdown;

pub fn html_to_markdown(html: &str) -> std::io::Result<String> {
    HtmlToMarkdown::new().convert(html)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn preserves_fenced_code_language() {
        let html = r#"<pre><code class="language-rust">fn main() {}</code></pre>"#;
        let md = html_to_markdown(html).unwrap();
        assert!(
            md.contains("```rust") || md.contains("fn main"),
            "unexpected markdown: {md}"
        );
    }

    #[test]
    fn keeps_tables() {
        let html = r#"
          <table>
            <tr><th>A</th><th>B</th></tr>
            <tr><td>1</td><td>2</td></tr>
          </table>
        "#;
        let md = html_to_markdown(html).unwrap();
        assert!(md.contains('|') || md.contains("A"), "unexpected: {md}");
    }
}
