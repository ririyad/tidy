use dom_smoothie::{Article, Metadata};

/// Hints from an earlier discovery step (usually a feed entry).
#[derive(Debug, Clone, Default)]
pub struct ArticleHints {
    pub title: Option<String>,
    pub author: Option<String>,
    pub published: Option<String>,
    pub excerpt: Option<String>,
}

#[derive(Debug, Clone, Default)]
pub struct PageMetadata {
    pub title: String,
    pub author: Option<String>,
    pub published: Option<String>,
    pub excerpt: Option<String>,
    pub lang: Option<String>,
    pub url: Option<String>,
    pub site_name: Option<String>,
    pub image: Option<String>,
}

/// Merge metadata with priority: feed hints > JSON-LD/OpenGraph meta > article body.
pub fn merge_metadata(hints: &ArticleHints, meta: &Metadata, article: &Article) -> PageMetadata {
    let title = first_nonempty([
        hints.title.as_deref(),
        non_empty(&meta.title),
        non_empty(&article.title),
    ])
    .unwrap_or_else(|| "Untitled".into());

    let author = first_owned([
        hints.author.clone(),
        meta.byline.clone(),
        article.byline.clone(),
    ]);

    let published = first_owned([
        hints.published.clone(),
        meta.published_time.clone(),
        article.published_time.clone(),
    ]);

    let excerpt = first_owned([
        hints.excerpt.clone(),
        meta.excerpt.clone(),
        article.excerpt.clone(),
    ]);

    PageMetadata {
        title,
        author,
        published,
        excerpt,
        lang: meta.lang.clone().or(article.lang.clone()),
        url: meta.url.clone().or(article.url.clone()),
        site_name: meta.site_name.clone().or(article.site_name.clone()),
        image: meta.image.clone().or(article.image.clone()),
    }
}

fn non_empty(value: &str) -> Option<&str> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed)
    }
}

fn first_nonempty<'a>(values: impl IntoIterator<Item = Option<&'a str>>) -> Option<String> {
    values.into_iter().flatten().map(str::to_owned).next()
}

fn first_owned(values: impl IntoIterator<Item = Option<String>>) -> Option<String> {
    values
        .into_iter()
        .flatten()
        .map(|value| value.trim().to_owned())
        .find(|value| !value.is_empty())
}
