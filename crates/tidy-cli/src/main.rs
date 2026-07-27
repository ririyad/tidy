use std::{env, path::PathBuf, process};

use anyhow::{Context, Result, bail};
use tidy_core::{
    ArticleFilter, ArticleQuery, CrawlLimits, DiscoverOptions, FetchOptions, Index, Vault,
    backup_vault, discover, fetch, list_highlights, list_run_history, parse_prefix, reindex_vault,
    schedule_status,
};

#[tokio::main]
async fn main() {
    if let Err(error) = run().await {
        eprintln!("error: {error:#}");
        process::exit(1);
    }
}

async fn run() -> Result<()> {
    let mut args = env::args().skip(1);
    let command = args.next().unwrap_or_else(|| "help".into());

    match command.as_str() {
        "init" => {
            let path = args
                .next()
                .map(PathBuf::from)
                .unwrap_or_else(|| PathBuf::from("."));
            let summary = Vault::initialize(&path)
                .with_context(|| format!("failed to initialize vault at {}", path.display()))?;

            if summary.created {
                println!("Created Tidy vault at {}", summary.path.display());
            } else {
                println!("Opened existing Tidy vault at {}", summary.path.display());
            }
            println!("Index: {}", summary.database_path.display());
            Ok(())
        }
        "discover" => {
            let prefix_arg = args
                .next()
                .context("usage: tidy discover <url-prefix> [--limit N] [--cache DIR]")?;

            let mut limit = None;
            let mut cache_dir = None;
            let mut json = false;

            while let Some(flag) = args.next() {
                match flag.as_str() {
                    "--limit" => {
                        let value = args.next().context("--limit requires a number")?;
                        limit = Some(value.parse::<usize>().context("invalid --limit")?);
                    }
                    "--cache" => {
                        let value = args.next().context("--cache requires a path")?;
                        cache_dir = Some(PathBuf::from(value));
                    }
                    "--json" => json = true,
                    other => bail!("unknown flag `{other}`"),
                }
            }

            let prefix = parse_prefix(&prefix_arg)
                .with_context(|| format!("invalid prefix `{prefix_arg}`"))?;

            let report = discover(DiscoverOptions {
                url_prefix: prefix,
                limit,
                cache_dir,
                limits: CrawlLimits::default(),
                overrides: Default::default(),
            })
            .await
            .context("discovery failed")?;

            if json {
                println!(
                    "{}",
                    serde_json::to_string_pretty(&report).context("failed to serialize report")?
                );
            } else {
                println!("prefix:          {}", report.prefix);
                println!("primary source:  {:?}", report.primary_source);
                if let Some(feed) = &report.feed_url {
                    println!("feed:            {feed}");
                }
                if !report.sitemap_urls.is_empty() {
                    println!(
                        "sitemaps:        {}",
                        report
                            .sitemap_urls
                            .iter()
                            .map(|u| u.as_str())
                            .collect::<Vec<_>>()
                            .join(", ")
                    );
                }
                println!("urls discovered: {}", report.urls.len());
                println!();
                for (index, item) in report.urls.iter().enumerate() {
                    let title = item.title.as_deref().unwrap_or("-");
                    println!(
                        "{:>4}. [{}] {}\n      {}",
                        index + 1,
                        format!("{:?}", item.source).to_lowercase(),
                        title,
                        item.url
                    );
                }
                if !report.warnings.is_empty() {
                    println!();
                    println!("warnings:");
                    for warning in &report.warnings {
                        println!("  - {warning}");
                    }
                }
            }
            Ok(())
        }
        "fetch" => {
            let prefix_arg = args.next().context(
                "usage: tidy fetch <url-prefix> --vault PATH [--limit N] [--no-images] [--json]",
            )?;

            let mut vault = None;
            let mut limit = None;
            let mut download_images = true;
            let mut json = false;

            while let Some(flag) = args.next() {
                match flag.as_str() {
                    "--vault" => {
                        let value = args.next().context("--vault requires a path")?;
                        vault = Some(PathBuf::from(value));
                    }
                    "--limit" => {
                        let value = args.next().context("--limit requires a number")?;
                        limit = Some(value.parse::<usize>().context("invalid --limit")?);
                    }
                    "--no-images" => download_images = false,
                    "--json" => json = true,
                    other => bail!("unknown flag `{other}`"),
                }
            }

            let vault = vault.context("--vault is required")?;
            let prefix = parse_prefix(&prefix_arg)
                .with_context(|| format!("invalid prefix `{prefix_arg}`"))?;

            let report = fetch(FetchOptions {
                url_prefix: prefix,
                vault,
                limit,
                download_images,
                title: None,
                backfill_policy: None,
                interval_minutes: None,
            })
            .await
            .context("fetch failed")?;

            if json {
                println!(
                    "{}",
                    serde_json::to_string_pretty(&report).context("failed to serialize report")?
                );
            } else {
                println!("source:       {}", report.source_slug);
                println!("discovered:   {}", report.discovered);
                println!(
                    "added/updated/skipped/failed: {}/{}/{}/{}",
                    report.added, report.updated, report.skipped, report.failed
                );
                if report.needs_review > 0 {
                    println!("needs review: {}", report.needs_review);
                }
                println!();
                for (index, item) in report.articles.iter().enumerate() {
                    println!(
                        "{:>4}. [{:?}] {} ({})\n      {}",
                        index + 1,
                        item.status,
                        item.title,
                        item.quality,
                        item.path
                    );
                }
                if !report.warnings.is_empty() {
                    println!();
                    println!("warnings:");
                    for warning in report.warnings.iter().take(20) {
                        println!("  - {warning}");
                    }
                    if report.warnings.len() > 20 {
                        println!("  … {} more", report.warnings.len() - 20);
                    }
                }
            }
            Ok(())
        }
        "schedule" => {
            let mut vault = None;
            let mut json = false;
            let mut runs = false;
            let mut source_id = None;
            let mut limit = 20i64;

            while let Some(flag) = args.next() {
                match flag.as_str() {
                    "--vault" => {
                        let value = args.next().context("--vault requires a path")?;
                        vault = Some(PathBuf::from(value));
                    }
                    "--json" => json = true,
                    "--runs" => runs = true,
                    "--source" => {
                        let value = args.next().context("--source requires an id")?;
                        source_id = Some(value.parse::<i64>().context("invalid --source")?);
                    }
                    "--limit" => {
                        let value = args.next().context("--limit requires a number")?;
                        limit = value.parse::<i64>().context("invalid --limit")?;
                    }
                    other => bail!("unknown flag `{other}`"),
                }
            }

            let vault_path = vault.context(
                "usage: tidy schedule --vault PATH [--runs] [--source ID] [--limit N] [--json]",
            )?;
            let vault = Vault::open(&vault_path)
                .with_context(|| format!("failed to open vault at {}", vault_path.display()))?;
            let index = Index::open(vault.database_path()).context("failed to open index")?;

            if runs {
                let history = list_run_history(&index, source_id, limit)
                    .context("failed to list fetch runs")?;
                if json {
                    println!(
                        "{}",
                        serde_json::to_string_pretty(&history)
                            .context("failed to serialize runs")?
                    );
                } else if history.is_empty() {
                    println!("No fetch runs yet.");
                } else {
                    for run in history {
                        println!(
                            "#{:<4} {:<20} {:<8} +{}/~{}/={}/!{}  {}",
                            run.id,
                            run.source_title,
                            run.status,
                            run.added,
                            run.updated,
                            run.skipped,
                            run.failed,
                            run.started_at
                        );
                    }
                }
            } else {
                let statuses = schedule_status(&index).context("failed to load schedule status")?;
                if json {
                    println!(
                        "{}",
                        serde_json::to_string_pretty(&statuses)
                            .context("failed to serialize schedule")?
                    );
                } else if statuses.is_empty() {
                    println!("No sources registered.");
                } else {
                    for item in statuses {
                        let due = if item.due { "due" } else { "ok" };
                        let enabled = if item.enabled { "on" } else { "off" };
                        println!(
                            "#{:<4} {:<24} every {:>4}m  [{enabled}] [{due}]  last {}",
                            item.source_id,
                            item.title,
                            item.interval_minutes,
                            item.last_fetch_at.as_deref().unwrap_or("never")
                        );
                    }
                }
            }
            Ok(())
        }
        "search" => {
            let mut vault = None;
            let mut filter = ArticleFilter::Inbox;
            let mut tag = None;
            let mut source_id = None;
            let mut limit = None;
            let mut json = false;
            let mut query_parts = Vec::new();

            while let Some(arg) = args.next() {
                match arg.as_str() {
                    "--vault" => {
                        let value = args.next().context("--vault requires a path")?;
                        vault = Some(PathBuf::from(value));
                    }
                    "--filter" => {
                        let value = args.next().context("--filter requires a value")?;
                        filter = match value.as_str() {
                            "unread" => ArticleFilter::Unread,
                            "starred" => ArticleFilter::Starred,
                            "archived" => ArticleFilter::Archived,
                            "all" => ArticleFilter::All,
                            _ => ArticleFilter::Inbox,
                        };
                    }
                    "--tag" => {
                        tag = Some(args.next().context("--tag requires a value")?);
                    }
                    "--source" => {
                        let value = args.next().context("--source requires an id")?;
                        source_id = Some(value.parse::<i64>().context("invalid --source")?);
                    }
                    "--limit" => {
                        let value = args.next().context("--limit requires a number")?;
                        limit = Some(value.parse::<i64>().context("invalid --limit")?);
                    }
                    "--json" => json = true,
                    other if other.starts_with('-') => bail!("unknown flag `{other}`"),
                    other => query_parts.push(other.to_string()),
                }
            }

            let vault_path = vault.context(
                "usage: tidy search --vault PATH [--filter inbox|starred|archived|all] [--tag TAG] [--source ID] [--limit N] [--json] [QUERY...]",
            )?;
            let search = if query_parts.is_empty() {
                None
            } else {
                Some(query_parts.join(" "))
            };

            let vault = Vault::open(&vault_path)
                .with_context(|| format!("failed to open vault at {}", vault_path.display()))?;
            let index = Index::open(vault.database_path()).context("failed to open index")?;
            let articles = index
                .query_articles(&ArticleQuery {
                    filter,
                    source_id,
                    tag,
                    search,
                    limit,
                })
                .context("search failed")?;

            if json {
                println!(
                    "{}",
                    serde_json::to_string_pretty(&articles)
                        .context("failed to serialize results")?
                );
            } else if articles.is_empty() {
                println!("No matching articles.");
            } else {
                for item in articles {
                    println!(
                        "#{:<6} {:<48} {}",
                        item.id,
                        truncate(&item.title, 48),
                        item.source_title
                    );
                }
            }
            Ok(())
        }
        "backup" => {
            let mut vault = None;
            let mut out = None;
            while let Some(flag) = args.next() {
                match flag.as_str() {
                    "--vault" => {
                        vault = Some(PathBuf::from(
                            args.next().context("--vault requires a path")?,
                        ));
                    }
                    "--out" => {
                        out = Some(PathBuf::from(args.next().context("--out requires a path")?));
                    }
                    other => bail!("unknown flag `{other}`"),
                }
            }
            let vault_path = vault.context("usage: tidy backup --vault PATH [--out DIR]")?;
            let out = out.unwrap_or_else(|| PathBuf::from("."));
            let vault = Vault::open(&vault_path)
                .with_context(|| format!("failed to open vault at {}", vault_path.display()))?;
            let report = backup_vault(&vault, out).context("backup failed")?;
            println!(
                "Backup written to {} ({} files)",
                report.destination.display(),
                report.copied_files
            );
            Ok(())
        }
        "reindex" => {
            let mut vault = None;
            while let Some(flag) = args.next() {
                match flag.as_str() {
                    "--vault" => {
                        vault = Some(PathBuf::from(
                            args.next().context("--vault requires a path")?,
                        ));
                    }
                    other => bail!("unknown flag `{other}`"),
                }
            }
            let vault_path = vault.context("usage: tidy reindex --vault PATH")?;
            let vault = Vault::open(&vault_path)
                .with_context(|| format!("failed to open vault at {}", vault_path.display()))?;
            let report = reindex_vault(&vault).context("reindex failed")?;
            println!(
                "Reindexed: scanned {}, upserted {}, skipped {}, failed {}",
                report.scanned, report.upserted, report.skipped, report.failed
            );
            for warning in report.warnings {
                eprintln!("warning: {warning}");
            }
            Ok(())
        }
        "highlights" => {
            let mut vault = None;
            let mut article_id = None;
            let mut json = false;

            while let Some(flag) = args.next() {
                match flag.as_str() {
                    "--vault" => {
                        let value = args.next().context("--vault requires a path")?;
                        vault = Some(PathBuf::from(value));
                    }
                    "--article" => {
                        let value = args.next().context("--article requires an id")?;
                        article_id = Some(value.parse::<i64>().context("invalid --article")?);
                    }
                    "--json" => json = true,
                    other => bail!("unknown flag `{other}`"),
                }
            }

            let vault_path =
                vault.context("usage: tidy highlights --vault PATH [--article ID] [--json]")?;
            let vault = Vault::open(&vault_path)
                .with_context(|| format!("failed to open vault at {}", vault_path.display()))?;
            let index = Index::open(vault.database_path()).context("failed to open index")?;
            let rows = list_highlights(&index, article_id).context("failed to list highlights")?;

            if json {
                println!(
                    "{}",
                    serde_json::to_string_pretty(&rows)
                        .context("failed to serialize highlights")?
                );
            } else if rows.is_empty() {
                println!("No highlights yet.");
            } else {
                for item in rows {
                    println!(
                        "{}  article#{}  \"{}\"{}",
                        item.id,
                        item.article_id,
                        truncate(&item.text, 60),
                        item.note
                            .as_ref()
                            .map(|note| format!("  — {}", truncate(note, 40)))
                            .unwrap_or_default()
                    );
                }
            }
            Ok(())
        }
        "help" | "--help" | "-h" => {
            print_help();
            Ok(())
        }
        other => bail!("unknown command `{other}`\n\n{HELP}"),
    }
}

fn print_help() {
    print!("{HELP}");
}

fn truncate(value: &str, max: usize) -> String {
    if value.chars().count() <= max {
        value.to_string()
    } else {
        format!(
            "{}…",
            value
                .chars()
                .take(max.saturating_sub(1))
                .collect::<String>()
        )
    }
}

const HELP: &str = "\
tidy — local-first fetch engine and vault tools

USAGE:
    tidy <COMMAND>

COMMANDS:
    init [PATH]                 Create or open a Tidy vault (default: .)
    discover <URL-PREFIX>       Enumerate posts under a URL prefix
        --limit N               Cap the number of printed URLs
        --cache DIR             HTTP cache directory
        --json                  Emit machine-readable JSON
    fetch <URL-PREFIX>          Discover, extract, and save articles
        --vault PATH            Vault directory (required)
        --limit N               Cap articles fetched
        --no-images             Skip downloading images
        --json                  Emit machine-readable JSON
    schedule --vault PATH       Show per-source intervals and due status
        --runs                  Show recent fetch run history
        --source ID             Filter runs to one source
        --limit N               Cap history rows (default 20)
        --json                  Emit machine-readable JSON
    search --vault PATH         Search indexed articles (FTS5)
        [--filter FILTER]       inbox, unread, starred, archived, or all
        [--tag TAG]             Require a tag (e.g. source/example)
        [--source ID]           Limit to one source
        [--limit N]             Cap result rows
        [--json]                Emit machine-readable JSON
        [QUERY...]              Full-text search terms
    highlights --vault PATH     List anchored highlights and notes
        [--article ID]          Limit to one article
        [--json]                Emit machine-readable JSON
    backup --vault PATH         Copy Sources/attachments/config to a folder
        [--out DIR]             Destination parent (default: .)
    reindex --vault PATH        Rebuild SQLite from markdown on disk
    help                        Show this help

Discovery order: feed → sitemap → HTML crawl fallback.
Extraction: readability → markdown + frontmatter → SQLite index.
Scheduler: due sources = enabled and (never fetched or interval elapsed).
";
