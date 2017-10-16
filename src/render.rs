use ammonia::{Builder, UrlRelative};
use comrak;
use std::borrow::Cow;
use url::Url;

use util::CargoResult;

/// Context for markdown to HTML rendering.
#[allow(missing_debug_implementations)]
pub struct MarkdownRenderer<'a> {
    html_sanitizer: Builder<'a>,
}

impl<'a> MarkdownRenderer<'a> {
    /// Creates a new renderer instance.
    ///
    /// Per `markdown_to_html`, `base_url` is the base URL prepended to any
    /// relative links in the input document.  See that function for more detail.
    fn new(base_url: Option<&'a str>) -> MarkdownRenderer<'a> {
        let tags = [
            "a",
            "b",
            "blockquote",
            "br",
            "code",
            "dd",
            "del",
            "dl",
            "dt",
            "em",
            "h1",
            "h2",
            "h3",
            "hr",
            "i",
            "img",
            "input",
            "kbd",
            "li",
            "ol",
            "p",
            "pre",
            "s",
            "strike",
            "strong",
            "sub",
            "sup",
            "table",
            "tbody",
            "td",
            "th",
            "thead",
            "tr",
            "ul",
            "hr",
            "span",
        ].iter()
            .cloned()
            .collect();
        let tag_attributes = [
            ("a", ["href", "target"].iter().cloned().collect()),
            (
                "img",
                ["width", "height", "src", "alt", "align"]
                    .iter()
                    .cloned()
                    .collect(),
            ),
            (
                "input",
                ["checked", "disabled", "type"].iter().cloned().collect(),
            ),
        ].iter()
            .cloned()
            .collect();
        let allowed_classes = [
            (
                "code",
                [
                    "language-bash",
                    "language-clike",
                    "language-glsl",
                    "language-go",
                    "language-ini",
                    "language-javascript",
                    "language-json",
                    "language-markup",
                    "language-protobuf",
                    "language-ruby",
                    "language-rust",
                    "language-scss",
                    "language-sql",
                    "yaml",
                ].iter()
                    .cloned()
                    .collect(),
            ),
        ].iter()
            .cloned()
            .collect();

        let sanitizer_base_url = base_url.map(|s| s.to_string());

        fn constrain_closure<F>(f: F) -> F
        where
            F: for<'a> Fn(&'a str) -> Option<Cow<'a, str>> + Send + Sync,
        {
            f
        }

        let relative_url_sanitizer = constrain_closure(move |url| {
            let mut new_url = sanitizer_base_url.clone().unwrap();
            if !new_url.ends_with('/') {
                new_url.push('/');
            }
            new_url += "blob/master";
            if !url.starts_with('/') {
                new_url.push('/');
            }
            new_url += url;
            Some(Cow::Owned(new_url))
        });

        let use_relative = if let Some(base_url) = base_url {
            if let Ok(url) = Url::parse(base_url) {
                url.host_str() == Some("github.com") || url.host_str() == Some("gitlab.com")
                    || url.host_str() == Some("bitbucket.org")
            } else {
                false
            }
        } else {
            false
        };

        let mut html_sanitizer = Builder::new();
        html_sanitizer
            .link_rel(Some("nofollow noopener noreferrer"))
            .tags(tags)
            .tag_attributes(tag_attributes)
            .allowed_classes(allowed_classes)
            .url_relative(if use_relative {
                UrlRelative::Custom(Box::new(relative_url_sanitizer))
            } else {
                UrlRelative::Deny
            });

        MarkdownRenderer {
            html_sanitizer: html_sanitizer,
        }
    }

    /// Renders the given markdown to HTML using the current settings.
    fn to_html(&self, text: &str) -> CargoResult<String> {
        let options = comrak::ComrakOptions {
            ext_autolink: true,
            ext_strikethrough: true,
            ext_table: true,
            ext_tagfilter: true,
            ext_tasklist: true,
            ..comrak::ComrakOptions::default()
        };
        let rendered = comrak::markdown_to_html(text, &options);
        Ok(self.html_sanitizer.clean(&rendered).to_string())
    }
}

/// Renders a markdown text to sanitized HTML.
///
/// The returned text should not contain any harmful HTML tag or attribute (such as iframe,
/// onclick, onmouseover, etc.).
///
/// The `base_url` parameter will be used as the base for any relative links found in the
/// Markdown, as long as its host part is github.com, gitlab.com, or bitbucket.org.  The
/// supplied URL will be used as a directory base whether or not the relative link is
/// prefixed with '/'.  If `None` is passed, relative links will be omitted.
///
/// # Examples
///
/// ```
/// use render::markdown_to_html;
///
/// let text = "[Rust](https://rust-lang.org/) is an awesome *systems programming* language!";
/// let rendered = markdown_to_html(text, None)?;
/// ```
pub fn markdown_to_html(text: &str, base_url: Option<&str>) -> CargoResult<String> {
    let renderer = MarkdownRenderer::new(base_url);
    renderer.to_html(text)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_text() {
        let text = "";
        let result = markdown_to_html(text, None).unwrap();
        assert_eq!(result, "");
    }

    #[test]
    fn text_with_script_tag() {
        let text = "foo_readme\n\n<script>alert('Hello World')</script>";
        let result = markdown_to_html(text, None).unwrap();
        assert_eq!(
            result,
            "<p>foo_readme</p>\n&lt;script&gt;alert(\'Hello World\')&lt;/script&gt;\n"
        );
    }

    #[test]
    fn text_with_iframe_tag() {
        let text = "foo_readme\n\n<iframe>alert('Hello World')</iframe>";
        let result = markdown_to_html(text, None).unwrap();
        assert_eq!(
            result,
            "<p>foo_readme</p>\n&lt;iframe&gt;alert(\'Hello World\')&lt;/iframe&gt;\n"
        );
    }

    #[test]
    fn text_with_unknown_tag() {
        let text = "foo_readme\n\n<unknown>alert('Hello World')</unknown>";
        let result = markdown_to_html(text, None).unwrap();
        assert_eq!(result, "<p>foo_readme</p>\n<p>alert(\'Hello World\')</p>\n");
    }

    #[test]
    fn text_with_inline_javascript() {
        let text = r#"foo_readme\n\n<a href="https://crates.io/crates/cargo-registry" onclick="window.alert('Got you')">Crate page</a>"#;
        let result = markdown_to_html(text, None).unwrap();
        assert_eq!(
            result,
            "<p>foo_readme\\n\\n<a href=\"https://crates.io/crates/cargo-registry\" rel=\"nofollow noopener noreferrer\">Crate page</a></p>\n"
        );
    }

    // See https://github.com/kivikakk/comrak/issues/37. This panic happened
    // in comrak 0.1.8 but was fixed in 0.1.9.
    #[test]
    fn text_with_fancy_single_quotes() {
        let text = r#"wb’"#;
        let result = markdown_to_html(text, None).unwrap();
        assert_eq!(result, "<p>wb’</p>\n");
    }

    #[test]
    fn code_block_with_syntax_highlighting() {
        let code_block = r#"```rust \
                            println!("Hello World"); \
                           ```"#;
        let result = markdown_to_html(code_block, None).unwrap();
        assert!(result.contains("<code class=\"language-rust\">"));
    }

    #[test]
    fn text_with_forbidden_class_attribute() {
        let text = "<p class='bad-class'>Hello World!</p>";
        let result = markdown_to_html(text, None).unwrap();
        assert_eq!(result, "<p>Hello World!</p>\n");
    }

    #[test]
    fn relative_links() {
        let absolute = "[hi](/hi)";
        let relative = "[there](there)";

        for host in &["github.com", "gitlab.com", "bitbucket.org"] {
            for &extra_slash in &[true, false] {
                let url = format!(
                    "https://{}/rust-lang/test{}",
                    host,
                    if extra_slash { "/" } else { "" }
                );

                let result = markdown_to_html(absolute, Some(&url)).unwrap();
                assert_eq!(
                    result,
                    format!(
                        "<p><a href=\"https://{}/rust-lang/test/blob/master/hi\" rel=\"nofollow noopener noreferrer\">hi</a></p>\n",
                        host
                    )
                );

                let result = markdown_to_html(relative, Some(&url)).unwrap();
                assert_eq!(
                    result,
                    format!(
                        "<p><a href=\"https://{}/rust-lang/test/blob/master/there\" rel=\"nofollow noopener noreferrer\">there</a></p>\n",
                        host
                    )
                );
            }
        }

        let result = markdown_to_html(absolute, Some("https://google.com/")).unwrap();
        assert_eq!(
            result,
            "<p><a rel=\"nofollow noopener noreferrer\">hi</a></p>\n"
        );
    }
}
