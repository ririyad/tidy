use std::{fs, path::PathBuf, sync::Mutex};

use serde::Serialize;
use tauri::{Emitter, Manager, State};
use tidy_core::{
    ArticleDetail, ArticleFilter, ArticleListItem, ArticleQuery, ArticleStatePatch, BackupReport,
    FetchOptions, FetchProgress, FetchReport, FetchRunRow, HighlightInput, HighlightRow, Index,
    ReaderSettings, ReindexReport, ScheduleStatus, SmartViewQuery, SmartViewRow, SourceOverrides,
    SourceRow, TagCount, Vault, VaultSummary, add_highlight, apply_article_state, backup_vault,
    delete_highlight, fetch_with_progress, list_due_sources as find_due_sources, list_highlights,
    list_run_history, load_reader_settings, parse_prefix, reindex_vault, save_reader_settings,
    schedule_status, source_slug, update_highlight_note,
};
use url::Url;

#[derive(Default)]
struct AppState {
    vault_path: Mutex<Option<PathBuf>>,
}

fn with_vault<T>(
    state: &AppState,
    f: impl FnOnce(&Vault, &Index) -> Result<T, String>,
) -> Result<T, String> {
    let guard = state.vault_path.lock().map_err(|e| e.to_string())?;
    let path = guard
        .as_ref()
        .ok_or_else(|| "no vault open — choose a vault first".to_string())?;
    let vault = Vault::open(path).map_err(|e| e.to_string())?;
    let index = Index::open(vault.database_path()).map_err(|e| e.to_string())?;
    f(&vault, &index)
}

fn last_vault_file(app: &tauri::AppHandle) -> Result<PathBuf, String> {
    let dir = app
        .path()
        .app_data_dir()
        .map_err(|error| format!("app data dir: {error}"))?;
    fs::create_dir_all(&dir).map_err(|error| error.to_string())?;
    Ok(dir.join("last_vault.txt"))
}

fn remember_vault(app: &tauri::AppHandle, path: &std::path::Path) -> Result<(), String> {
    let file = last_vault_file(app)?;
    fs::write(&file, path.display().to_string()).map_err(|error| error.to_string())
}

#[tauri::command]
fn select_vault(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
) -> Result<Option<VaultSummary>, String> {
    use tauri_plugin_dialog::DialogExt;

    let path = app
        .dialog()
        .file()
        .set_title("Choose a Tidy vault folder")
        .blocking_pick_folder();

    let Some(file_path) = path else {
        return Ok(None);
    };

    let path = file_path
        .into_path()
        .map_err(|error| format!("invalid vault path: {error}"))?;

    let summary = Vault::initialize(&path).map_err(|e| e.to_string())?;
    *state.vault_path.lock().map_err(|e| e.to_string())? = Some(summary.path.clone());
    remember_vault(&app, &summary.path)?;
    Ok(Some(summary))
}

#[tauri::command]
fn open_vault_path(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
    path: String,
) -> Result<VaultSummary, String> {
    let summary = Vault::initialize(PathBuf::from(path)).map_err(|e| e.to_string())?;
    *state.vault_path.lock().map_err(|e| e.to_string())? = Some(summary.path.clone());
    remember_vault(&app, &summary.path)?;
    Ok(summary)
}

#[tauri::command]
fn get_last_vault_path(app: tauri::AppHandle) -> Result<Option<String>, String> {
    let file = last_vault_file(&app)?;
    if !file.exists() {
        return Ok(None);
    }
    let path = fs::read_to_string(&file).map_err(|error| error.to_string())?;
    let path = path.trim();
    if path.is_empty() || !PathBuf::from(path).is_dir() {
        return Ok(None);
    }
    Ok(Some(path.to_owned()))
}

#[tauri::command]
fn get_open_vault(state: State<'_, AppState>) -> Result<Option<String>, String> {
    let guard = state.vault_path.lock().map_err(|e| e.to_string())?;
    Ok(guard.as_ref().map(|p| p.display().to_string()))
}

#[tauri::command]
fn list_sources(state: State<'_, AppState>) -> Result<Vec<SourceRow>, String> {
    with_vault(&state, |_, index| {
        index.list_sources().map_err(|e| e.to_string())
    })
}

#[derive(Debug, serde::Deserialize)]
struct AddSourceRequest {
    url_prefix: String,
    title: Option<String>,
    backfill: String, // "recent" | "full"
    recent_limit: Option<usize>,
    interval_minutes: Option<i64>,
}

#[tauri::command]
async fn add_source(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
    request: AddSourceRequest,
) -> Result<FetchReport, String> {
    let vault_path = {
        let guard = state.vault_path.lock().map_err(|e| e.to_string())?;
        guard.clone().ok_or_else(|| "no vault open".to_string())?
    };

    let prefix = parse_prefix(&request.url_prefix).map_err(|e| e.to_string())?;
    let limit = match request.backfill.as_str() {
        "full" => None,
        _ => Some(request.recent_limit.unwrap_or(20)),
    };
    let title = request
        .title
        .filter(|t| !t.trim().is_empty())
        .unwrap_or_else(|| source_slug(&prefix));

    let app_handle = app.clone();
    let report = fetch_with_progress(
        FetchOptions {
            url_prefix: prefix,
            vault: vault_path,
            limit,
            download_images: true,
            title: Some(title),
            backfill_policy: Some(request.backfill),
            interval_minutes: request.interval_minutes,
        },
        move |progress: FetchProgress| {
            let _ = app_handle.emit("fetch-progress", &progress);
        },
    )
    .await
    .map_err(|e| e.to_string())?;

    Ok(report)
}

#[tauri::command]
async fn refresh_source(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
    source_id: i64,
    limit: Option<usize>,
) -> Result<FetchReport, String> {
    let (vault_path, prefix, title, backfill) = with_vault(&state, |vault, index| {
        let source = index
            .get_source(source_id)
            .map_err(|e| e.to_string())?
            .ok_or_else(|| format!("source {source_id} not found"))?;
        Ok((
            vault.root().to_path_buf(),
            source.url_prefix,
            source.title,
            source.backfill_policy,
        ))
    })?;

    let prefix = Url::parse(&prefix)
        .or_else(|_| parse_prefix(&prefix).map_err(|e| e.to_string()))
        .map_err(|e| e.to_string())?;

    let limit = limit.or_else(|| if backfill == "full" { None } else { Some(20) });

    let app_handle = app.clone();
    fetch_with_progress(
        FetchOptions {
            url_prefix: prefix,
            vault: vault_path,
            limit,
            download_images: true,
            title: Some(title),
            backfill_policy: Some(backfill),
            interval_minutes: None,
        },
        move |progress: FetchProgress| {
            let _ = app_handle.emit("fetch-progress", &progress);
        },
    )
    .await
    .map_err(|e| e.to_string())
}

#[tauri::command]
fn update_source_schedule(
    state: State<'_, AppState>,
    source_id: i64,
    interval_minutes: Option<i64>,
    enabled: Option<bool>,
) -> Result<SourceRow, String> {
    with_vault(&state, |_, index| {
        index
            .update_source_schedule(source_id, interval_minutes, enabled)
            .map_err(|e| e.to_string())?
            .ok_or_else(|| format!("source {source_id} not found"))
    })
}

#[tauri::command]
fn list_due_sources(state: State<'_, AppState>) -> Result<Vec<SourceRow>, String> {
    with_vault(&state, |_, index| {
        find_due_sources(index).map_err(|e| e.to_string())
    })
}

#[tauri::command]
fn get_schedule_status(state: State<'_, AppState>) -> Result<Vec<ScheduleStatus>, String> {
    with_vault(&state, |_, index| {
        schedule_status(index).map_err(|e| e.to_string())
    })
}

#[tauri::command]
fn list_fetch_runs(
    state: State<'_, AppState>,
    source_id: Option<i64>,
    limit: Option<i64>,
) -> Result<Vec<FetchRunRow>, String> {
    with_vault(&state, |_, index| {
        list_run_history(index, source_id, limit.unwrap_or(25)).map_err(|e| e.to_string())
    })
}

/// Refresh every source whose interval has elapsed (launch catch-up / background tick).
#[tauri::command]
async fn catch_up_due_sources(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
) -> Result<Vec<FetchReport>, String> {
    let due = with_vault(&state, |_, index| {
        find_due_sources(index).map_err(|e| e.to_string())
    })?;

    let mut reports = Vec::new();
    for source in due {
        let report = refresh_source(app.clone(), state.clone(), source.id, None).await?;
        reports.push(report);
    }
    Ok(reports)
}

#[tauri::command]
fn remove_source(state: State<'_, AppState>, source_id: i64) -> Result<bool, String> {
    with_vault(&state, |_, index| {
        index.delete_source(source_id).map_err(|e| e.to_string())
    })
}

#[derive(Debug, serde::Deserialize)]
struct ListArticlesRequest {
    filter: Option<String>,
    source_id: Option<i64>,
    limit: Option<i64>,
    search: Option<String>,
    tag: Option<String>,
}

#[tauri::command]
fn list_articles(
    state: State<'_, AppState>,
    request: ListArticlesRequest,
) -> Result<Vec<ArticleListItem>, String> {
    let filter = match request.filter.as_deref().unwrap_or("inbox") {
        "unread" => ArticleFilter::Unread,
        "starred" => ArticleFilter::Starred,
        "archived" => ArticleFilter::Archived,
        "all" => ArticleFilter::All,
        "review" => ArticleFilter::Review,
        _ => ArticleFilter::Inbox,
    };
    let query = ArticleQuery {
        filter,
        source_id: request.source_id,
        tag: request.tag,
        search: request.search,
        limit: request.limit,
    };
    with_vault(&state, |_, index| {
        index.query_articles(&query).map_err(|e| e.to_string())
    })
}

#[tauri::command]
fn list_tags(
    state: State<'_, AppState>,
    prefix: Option<String>,
    limit: Option<i64>,
) -> Result<Vec<TagCount>, String> {
    with_vault(&state, |_, index| {
        index
            .list_tags(prefix.as_deref(), limit.unwrap_or(50))
            .map_err(|e| e.to_string())
    })
}

#[derive(Debug, serde::Deserialize)]
struct SaveSmartViewRequest {
    id: Option<String>,
    name: String,
    query: SmartViewQuery,
}

#[tauri::command]
fn list_smart_views(state: State<'_, AppState>) -> Result<Vec<SmartViewRow>, String> {
    with_vault(&state, |_, index| {
        index.list_smart_views().map_err(|e| e.to_string())
    })
}

#[tauri::command]
fn save_smart_view(
    state: State<'_, AppState>,
    request: SaveSmartViewRequest,
) -> Result<SmartViewRow, String> {
    with_vault(&state, |_, index| {
        index
            .save_smart_view(request.id.as_deref(), &request.name, &request.query)
            .map_err(|e| e.to_string())
    })
}

#[tauri::command]
fn delete_smart_view(state: State<'_, AppState>, id: String) -> Result<bool, String> {
    with_vault(&state, |_, index| {
        index.delete_smart_view(&id).map_err(|e| e.to_string())
    })
}

#[tauri::command]
fn get_article(state: State<'_, AppState>, id: i64) -> Result<Option<ArticleDetail>, String> {
    with_vault(&state, |_, index| {
        index.get_article(id).map_err(|e| e.to_string())
    })
}

#[derive(Debug, serde::Deserialize)]
struct AddHighlightRequest {
    article_id: i64,
    text: String,
    note: Option<String>,
    prefix: Option<String>,
    suffix: Option<String>,
}

#[tauri::command]
fn add_article_highlight(
    state: State<'_, AppState>,
    request: AddHighlightRequest,
) -> Result<HighlightRow, String> {
    with_vault(&state, |vault, index| {
        add_highlight(
            vault,
            index,
            request.article_id,
            HighlightInput {
                text: request.text,
                note: request.note,
                prefix: request.prefix,
                suffix: request.suffix,
            },
        )
        .map_err(|e| e.to_string())
    })
}

#[tauri::command]
fn update_article_highlight_note(
    state: State<'_, AppState>,
    id: String,
    note: Option<String>,
) -> Result<HighlightRow, String> {
    with_vault(&state, |vault, index| {
        update_highlight_note(vault, index, &id, note).map_err(|e| e.to_string())
    })
}

#[tauri::command]
fn delete_article_highlight(state: State<'_, AppState>, id: String) -> Result<bool, String> {
    with_vault(&state, |vault, index| {
        delete_highlight(vault, index, &id).map_err(|e| e.to_string())
    })
}

#[tauri::command]
fn list_article_highlights(
    state: State<'_, AppState>,
    article_id: Option<i64>,
) -> Result<Vec<HighlightRow>, String> {
    with_vault(&state, |_, index| {
        list_highlights(index, article_id).map_err(|e| e.to_string())
    })
}

#[derive(Debug, serde::Deserialize)]
struct SetArticleStateRequest {
    id: i64,
    state: Option<String>,
    starred: Option<bool>,
    archived: Option<bool>,
}

#[tauri::command]
fn set_article_state(
    state: State<'_, AppState>,
    request: SetArticleStateRequest,
) -> Result<ArticleDetail, String> {
    with_vault(&state, |vault, index| {
        apply_article_state(
            vault,
            index,
            request.id,
            ArticleStatePatch {
                state: request.state,
                starred: request.starred,
                archived: request.archived,
            },
        )
        .map_err(|e| e.to_string())
    })
}

#[tauri::command]
fn set_article_progress(state: State<'_, AppState>, id: i64, progress: f64) -> Result<(), String> {
    with_vault(&state, |_, index| {
        index
            .update_article_progress(id, progress)
            .map_err(|e| e.to_string())
    })
}

#[tauri::command]
fn get_reader_settings(state: State<'_, AppState>) -> Result<ReaderSettings, String> {
    with_vault(&state, |vault, _| {
        load_reader_settings(vault).map_err(|e| e.to_string())
    })
}

#[tauri::command]
fn set_reader_settings(
    state: State<'_, AppState>,
    settings: ReaderSettings,
) -> Result<ReaderSettings, String> {
    with_vault(&state, |vault, _| {
        save_reader_settings(vault, &settings).map_err(|e| e.to_string())?;
        Ok(settings)
    })
}

#[tauri::command]
fn update_source_overrides(
    state: State<'_, AppState>,
    source_id: i64,
    overrides: SourceOverrides,
) -> Result<SourceRow, String> {
    with_vault(&state, |_, index| {
        index
            .update_source_overrides(source_id, &overrides)
            .map_err(|e| e.to_string())?
            .ok_or_else(|| format!("source {source_id} not found"))
    })
}

#[tauri::command]
fn backup_open_vault(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
) -> Result<Option<BackupReport>, String> {
    use tauri_plugin_dialog::DialogExt;

    let destination = app
        .dialog()
        .file()
        .set_title("Choose a folder for the vault backup")
        .blocking_pick_folder();
    let Some(file_path) = destination else {
        return Ok(None);
    };
    let parent = file_path
        .into_path()
        .map_err(|error| format!("invalid backup path: {error}"))?;

    with_vault(&state, |vault, _| {
        backup_vault(vault, parent)
            .map(Some)
            .map_err(|e| e.to_string())
    })
}

#[tauri::command]
fn reindex_open_vault(state: State<'_, AppState>) -> Result<ReindexReport, String> {
    with_vault(&state, |vault, _| {
        reindex_vault(vault).map_err(|e| e.to_string())
    })
}

#[derive(Serialize)]
struct AppInfo {
    name: String,
    version: String,
}

#[tauri::command]
fn get_app_info() -> AppInfo {
    AppInfo {
        name: tidy_core::APP_NAME.to_string(),
        version: env!("CARGO_PKG_VERSION").to_string(),
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .manage(AppState {
            vault_path: Mutex::new(None),
        })
        .invoke_handler(tauri::generate_handler![
            select_vault,
            open_vault_path,
            get_last_vault_path,
            get_open_vault,
            list_sources,
            add_source,
            refresh_source,
            remove_source,
            update_source_schedule,
            update_source_overrides,
            list_due_sources,
            get_schedule_status,
            list_fetch_runs,
            catch_up_due_sources,
            list_articles,
            list_tags,
            list_smart_views,
            save_smart_view,
            delete_smart_view,
            get_article,
            add_article_highlight,
            update_article_highlight_note,
            delete_article_highlight,
            list_article_highlights,
            set_article_state,
            set_article_progress,
            get_reader_settings,
            set_reader_settings,
            backup_open_vault,
            reindex_open_vault,
            get_app_info
        ])
        .run(tauri::generate_context!())
        .expect("error while running Tidy");
}
