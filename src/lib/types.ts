export type VaultSummary = {
  path: string;
  database_path: string;
  created: boolean;
};

export type SourceRow = {
  id: number;
  url_prefix: string;
  title: string;
  feed_url: string | null;
  backfill_policy: string;
  interval_minutes: number;
  last_fetch_at: string | null;
  enabled: boolean;
  article_count: number;
  unread_count: number;
  overrides: SourceOverrides;
};

export type SourceOverrides = {
  content_selector?: string | null;
  title_selector?: string | null;
  pagination_link_selector?: string | null;
  max_pages?: number | null;
};

export type FetchRunRow = {
  id: number;
  source_id: number;
  source_title: string;
  started_at: string;
  finished_at: string | null;
  status: string;
  discovered: number;
  added: number;
  updated: number;
  skipped: number;
  failed: number;
};

export type ScheduleStatus = {
  source_id: number;
  title: string;
  interval_minutes: number;
  last_fetch_at: string | null;
  enabled: boolean;
  due: boolean;
  next_fetch_at: string | null;
};

export type ArticleListItem = {
  id: number;
  source_id: number;
  source_title: string;
  url: string;
  path: string;
  title: string;
  author: string | null;
  published_at: string | null;
  fetched_at: string;
  word_count: number;
  reading_time: number;
  excerpt: string;
  state: string;
  starred: boolean;
  archived: boolean;
  quality: string;
};

export type ArticleDetail = ArticleListItem & {
  body: string;
  rendered_html: string;
  progress: number;
  revision: number;
  highlights: HighlightRow[];
};

export type HighlightRow = {
  id: string;
  article_id: number;
  text: string;
  note: string | null;
  prefix: string;
  suffix: string;
  created_at: string;
};

export type ReaderSettings = {
  theme: 'paper' | 'ink' | 'sepia' | string;
  font: 'serif' | 'sans' | string;
  font_size: number;
  line_height: number;
  measure: 'narrow' | 'wide' | string;
};

export type FetchProgress = {
  source_id: number;
  phase: string;
  current: number;
  total: number;
  url: string | null;
  title: string | null;
  message: string;
};

export type FeedFilter = 'inbox' | 'unread' | 'starred' | 'archived' | 'all' | 'review';

export type BackupReport = {
  destination: string;
  copied_files: number;
};

export type ReindexReport = {
  scanned: number;
  upserted: number;
  skipped: number;
  failed: number;
  warnings: string[];
};

export type AppInfo = {
  name: string;
  version: string;
};

export type TagCount = {
  tag: string;
  count: number;
};

export type SmartViewQuery = {
  filter: FeedFilter;
  tag?: string | null;
  query?: string | null;
  source_id?: number | null;
};

export type SmartViewRow = {
  id: string;
  name: string;
  query: SmartViewQuery;
  position: number;
};

export type ArticleQuery = {
  filter: FeedFilter;
  source_id?: number | null;
  tag?: string | null;
  search?: string | null;
  limit?: number | null;
};

export type DayGroup = {
  label: string;
  key: string;
  articles: ArticleListItem[];
};
