use url::Url;

use crate::error::{Result, TidyError};

/// Normalize a URL prefix so path comparisons are stable.
/// Ensures a trailing slash on the path (except bare origin where we keep `/`).
pub fn normalize_prefix(url: &Url) -> Result<Url> {
    let mut normalized = url.clone();
    normalized.set_fragment(None);
    if normalized.query().is_some() {
        normalized.set_query(None);
    }

    let path = normalized.path();
    if path.is_empty() {
        normalized.set_path("/");
    } else if !path.ends_with('/') {
        // Keep file-like endings as-is only if they look like documents;
        // for directory prefixes ensure trailing slash.
        let last = path.rsplit('/').next().unwrap_or("");
        if !last.contains('.') {
            let mut with_slash = path.to_owned();
            with_slash.push('/');
            normalized.set_path(&with_slash);
        }
    }

    if normalized.host_str().is_none() {
        return Err(TidyError::Message(format!(
            "prefix has no host: {normalized}"
        )));
    }

    Ok(normalized)
}

/// True when `candidate` is same-host and its path is under the prefix path.
pub fn matches_prefix(candidate: &Url, prefix: &Url) -> bool {
    if candidate.scheme() != prefix.scheme() {
        return false;
    }
    if candidate.host_str() != prefix.host_str() {
        return false;
    }
    if candidate.port_or_known_default() != prefix.port_or_known_default() {
        return false;
    }

    let prefix_path = prefix.path();
    let candidate_path = candidate.path();

    if prefix_path == "/" {
        return true;
    }

    candidate_path == prefix_path.trim_end_matches('/') || candidate_path.starts_with(prefix_path)
}

/// True when the candidate is the prefix index itself (not an article under it).
pub fn is_prefix_root(candidate: &Url, prefix: &Url) -> bool {
    if !matches_prefix(candidate, prefix) {
        return false;
    }
    let left = candidate.path().trim_end_matches('/');
    let right = prefix.path().trim_end_matches('/');
    left == right || (right.is_empty() && (left.is_empty() || left == "/"))
}

pub fn same_host(a: &Url, b: &Url) -> bool {
    a.scheme() == b.scheme()
        && a.host_str() == b.host_str()
        && a.port_or_known_default() == b.port_or_known_default()
}

/// Drop common campaign/tracking query params so the same article collapses.
pub fn scrub_tracking_params(url: &mut Url) {
    let pairs: Vec<(String, String)> = url
        .query_pairs()
        .map(|(k, v)| (k.into_owned(), v.into_owned()))
        .collect();
    if pairs.is_empty() {
        return;
    }

    let filtered: Vec<(String, String)> = pairs
        .into_iter()
        .filter(|(key, _)| {
            let lower = key.to_ascii_lowercase();
            !(lower.starts_with("utm_")
                || lower == "fbclid"
                || lower == "gclid"
                || lower == "mc_cid"
                || lower == "mc_eid")
        })
        .collect();

    if filtered.is_empty() {
        url.set_query(None);
    } else {
        let mut serializer = url::form_urlencoded::Serializer::new(String::new());
        for (key, value) in &filtered {
            serializer.append_pair(key, value);
        }
        let query = serializer.finish();
        url.set_query(Some(&query));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn prefix_matching_is_path_aware() {
        let prefix = normalize_prefix(&Url::parse("https://example.com/blog").unwrap()).unwrap();
        assert!(matches_prefix(
            &Url::parse("https://example.com/blog/hello").unwrap(),
            &prefix
        ));
        assert!(matches_prefix(
            &Url::parse("https://example.com/blog").unwrap(),
            &prefix
        ));
        assert!(!matches_prefix(
            &Url::parse("https://example.com/blogger/x").unwrap(),
            &prefix
        ));
        assert!(!matches_prefix(
            &Url::parse("https://other.com/blog/x").unwrap(),
            &prefix
        ));
        assert!(is_prefix_root(
            &Url::parse("https://example.com/blog").unwrap(),
            &prefix
        ));
        assert!(!is_prefix_root(
            &Url::parse("https://example.com/blog/hello").unwrap(),
            &prefix
        ));
    }
}
