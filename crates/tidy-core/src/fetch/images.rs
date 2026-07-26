use std::{fs, path::PathBuf};

use url::Url;

use crate::error::{Result, TidyError};
use crate::http::HttpClient;
use crate::vault::Vault;

use super::slug::short_hash;

/// Download remote images referenced by the markdown and rewrite to vault-relative paths.
pub async fn localize_images(
    client: &HttpClient,
    vault: &Vault,
    source_slug: &str,
    article_stem: &str,
    page_url: &Url,
    markdown: &str,
) -> Result<String> {
    let mut output = String::with_capacity(markdown.len());
    let bytes = markdown.as_bytes();
    let mut index = 0usize;
    let mut image_index = 0usize;

    while index < bytes.len() {
        if let Some(rel) = find_markdown_image(&markdown[index..]) {
            let absolute_start = index + rel.start;
            let absolute_end = index + rel.end;
            output.push_str(&markdown[index..absolute_start]);

            let raw_url = &markdown[index + rel.url_start..index + rel.url_end];
            let alt_text = &markdown[index + rel.start + 2..index + rel.alt_end];
            match resolve_and_store(
                client,
                vault,
                source_slug,
                article_stem,
                page_url,
                raw_url,
                image_index,
            )
            .await
            {
                Ok(local) => {
                    output.push_str("![");
                    output.push_str(alt_text);
                    output.push_str("](");
                    output.push_str(&local);
                    output.push(')');
                    image_index += 1;
                }
                Err(_) => {
                    // Keep the original reference if download fails.
                    output.push_str(&markdown[absolute_start..absolute_end]);
                }
            }
            index = absolute_end;
        } else {
            output.push_str(&markdown[index..]);
            break;
        }
    }

    Ok(output)
}

struct ImageMatch {
    start: usize,
    end: usize,
    alt_end: usize,
    url_start: usize,
    url_end: usize,
}

fn find_markdown_image(input: &str) -> Option<ImageMatch> {
    let bytes = input.as_bytes();
    let mut i = 0;
    while i + 1 < bytes.len() {
        if bytes[i] == b'!' && bytes[i + 1] == b'[' {
            let alt_start = i + 2;
            let Some(alt_end_rel) = input[alt_start..].find(']') else {
                i += 1;
                continue;
            };
            let alt_end = alt_start + alt_end_rel;
            if alt_end + 1 >= bytes.len() || bytes[alt_end + 1] != b'(' {
                i += 1;
                continue;
            }
            let url_start = alt_end + 2;
            let Some(url_end_rel) = input[url_start..].find(')') else {
                i += 1;
                continue;
            };
            let url_end = url_start + url_end_rel;
            return Some(ImageMatch {
                start: i,
                end: url_end + 1,
                alt_end,
                url_start,
                url_end,
            });
        }
        i += 1;
    }
    None
}

async fn resolve_and_store(
    client: &HttpClient,
    vault: &Vault,
    source_slug: &str,
    article_stem: &str,
    page_url: &Url,
    raw_url: &str,
    image_index: usize,
) -> Result<String> {
    let trimmed = raw_url.trim();
    if trimmed.starts_with("data:") {
        return Err(TidyError::Message("skip data URI".into()));
    }

    let absolute = page_url
        .join(trimmed)
        .map_err(|error| TidyError::Message(error.to_string()))?;

    if absolute.scheme() != "http" && absolute.scheme() != "https" {
        return Err(TidyError::Message("unsupported image scheme".into()));
    }

    let response = client.get_bytes(&absolute).await?;
    let extension = extension_for(&absolute, response.content_type.as_deref());
    let file_stem = {
        let from_url = absolute
            .path_segments()
            .and_then(|mut parts| parts.next_back())
            .unwrap_or("image");
        let cleaned = sanitize_filename(from_url);
        let base = if cleaned.is_empty() {
            format!("image-{image_index}")
        } else {
            PathBuf::from(&cleaned)
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("image")
                .to_owned()
        };
        // Include a stable URL hash so re-fetches rewrite to the same path.
        format!("{base}-{}", short_hash(absolute.as_str()))
    };

    let filename = format!("{file_stem}.{extension}");
    let relative_dir = format!("attachments/{source_slug}/{article_stem}");
    let absolute_dir = vault.root().join(&relative_dir);
    fs::create_dir_all(&absolute_dir)?;

    let absolute_file = absolute_dir.join(&filename);
    if !absolute_file.exists() {
        let tmp = absolute_file.with_extension(format!("{extension}.tmp"));
        fs::write(&tmp, &response.body)?;
        fs::rename(&tmp, &absolute_file)?;
    }

    let file_name = absolute_file
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or(&filename);
    // From Sources/<source>/<article>.md → ../../attachments/...
    Ok(format!(
        "../../attachments/{source_slug}/{article_stem}/{file_name}"
    ))
}

fn extension_for(url: &Url, content_type: Option<&str>) -> String {
    if let Some(ctype) = content_type {
        let mime = ctype
            .split(';')
            .next()
            .unwrap_or("")
            .trim()
            .to_ascii_lowercase();
        match mime.as_str() {
            "image/jpeg" | "image/jpg" => return "jpg".into(),
            "image/png" => return "png".into(),
            "image/gif" => return "gif".into(),
            "image/webp" => return "webp".into(),
            "image/svg+xml" => return "svg".into(),
            "image/avif" => return "avif".into(),
            _ => {}
        }
    }

    url.path()
        .rsplit('.')
        .next()
        .map(|ext| ext.to_ascii_lowercase())
        .filter(|ext| {
            matches!(
                ext.as_str(),
                "jpg" | "jpeg" | "png" | "gif" | "webp" | "svg" | "avif" | "bmp"
            )
        })
        .map(|ext| if ext == "jpeg" { "jpg".into() } else { ext })
        .unwrap_or_else(|| "bin".into())
}

fn sanitize_filename(input: &str) -> String {
    input
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() || ch == '-' || ch == '_' || ch == '.' {
                ch
            } else {
                '-'
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn finds_image_syntax() {
        let input = "hello ![Alt text](https://cdn.example/a.png) world";
        let found = find_markdown_image(input).unwrap();
        assert_eq!(
            &input[found.url_start..found.url_end],
            "https://cdn.example/a.png"
        );
    }
}
