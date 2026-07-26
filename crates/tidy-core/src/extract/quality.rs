#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExtractionQuality {
    Ok,
    NeedsReview,
}

const MIN_WORDS: usize = 40;

const JS_SHELL_MARKERS: &[&str] = &[
    "enable javascript",
    "enable js",
    "requires javascript",
    "you need to enable javascript",
    "this page requires javascript",
];

pub fn assess_quality(raw_html: &str, text: &str, word_count: usize) -> ExtractionQuality {
    if word_count < MIN_WORDS {
        return ExtractionQuality::NeedsReview;
    }

    let lower_text = text.to_ascii_lowercase();
    if JS_SHELL_MARKERS
        .iter()
        .any(|marker| lower_text.contains(marker))
    {
        return ExtractionQuality::NeedsReview;
    }

    let lower_html = raw_html.to_ascii_lowercase();
    let looks_like_empty_spa =
        lower_html.contains("id=\"root\"") && !lower_html.contains("<article") && word_count < 80;
    if looks_like_empty_spa {
        return ExtractionQuality::NeedsReview;
    }

    ExtractionQuality::Ok
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn flags_thin_content() {
        assert_eq!(
            assess_quality("<html></html>", "too short", 2),
            ExtractionQuality::NeedsReview
        );
    }
}
