import { invoke } from '@tauri-apps/api/core';
import type {
  ArticleDetail,
  ArticleListItem,
  ArticleQuery,
  FeedFilter,
  FetchRunRow,
  ReaderSettings,
  ScheduleStatus,
  SmartViewQuery,
  SmartViewRow,
  SourceRow,
  TagCount,
  VaultSummary
} from './types';

export const api = {
  selectVault: () => invoke<VaultSummary | null>('select_vault'),
  listSources: () => invoke<SourceRow[]>('list_sources'),
  addSource: (payload: {
    url_prefix: string;
    title?: string;
    backfill: 'recent' | 'full';
    recent_limit?: number;
    interval_minutes?: number;
  }) => invoke('add_source', { request: payload }),
  refreshSource: (sourceId: number, limit?: number) =>
    invoke('refresh_source', { sourceId, limit }),
  removeSource: (sourceId: number) => invoke<boolean>('remove_source', { sourceId }),
  updateSourceSchedule: (payload: {
    sourceId: number;
    intervalMinutes?: number;
    enabled?: boolean;
  }) =>
    invoke<SourceRow>('update_source_schedule', {
      sourceId: payload.sourceId,
      intervalMinutes: payload.intervalMinutes ?? null,
      enabled: payload.enabled ?? null
    }),
  listDueSources: () => invoke<SourceRow[]>('list_due_sources'),
  getScheduleStatus: () => invoke<ScheduleStatus[]>('get_schedule_status'),
  listFetchRuns: (sourceId?: number | null, limit = 25) =>
    invoke<FetchRunRow[]>('list_fetch_runs', {
      sourceId: sourceId ?? null,
      limit
    }),
  catchUpDueSources: () => invoke('catch_up_due_sources'),
  listArticles: (filter: FeedFilter, sourceId?: number | null) =>
    invoke<ArticleListItem[]>('list_articles', {
      request: {
        filter,
        source_id: sourceId ?? null,
        limit: null,
        search: null,
        tag: null
      }
    }),
  queryArticles: (query: ArticleQuery) =>
    invoke<ArticleListItem[]>('list_articles', {
      request: {
        filter: query.filter,
        source_id: query.source_id ?? null,
        limit: query.limit ?? null,
        search: query.search ?? null,
        tag: query.tag ?? null
      }
    }),
  listTags: (prefix?: string | null, limit = 50) =>
    invoke<TagCount[]>('list_tags', { prefix: prefix ?? null, limit }),
  listSmartViews: () => invoke<SmartViewRow[]>('list_smart_views'),
  saveSmartView: (payload: { id?: string; name: string; query: SmartViewQuery }) =>
    invoke<SmartViewRow>('save_smart_view', { request: payload }),
  deleteSmartView: (id: string) => invoke<boolean>('delete_smart_view', { id }),
  getArticle: (id: number) => invoke<ArticleDetail | null>('get_article', { id }),
  setArticleState: (payload: {
    id: number;
    state?: string;
    starred?: boolean;
    archived?: boolean;
  }) => invoke<ArticleDetail>('set_article_state', { request: payload }),
  setArticleProgress: (id: number, progress: number) =>
    invoke('set_article_progress', { id, progress }),
  getReaderSettings: () => invoke<ReaderSettings>('get_reader_settings'),
  setReaderSettings: (settings: ReaderSettings) =>
    invoke<ReaderSettings>('set_reader_settings', { settings })
};
