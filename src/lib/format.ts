import type { ArticleListItem, DayGroup } from './types';

export function groupByDay(articles: ArticleListItem[]): DayGroup[] {
  const groups = new Map<string, DayGroup>();

  for (const article of articles) {
    const raw = article.published_at || article.fetched_at;
    const date = new Date(raw);
    const key = Number.isNaN(date.getTime())
      ? 'unknown'
      : date.toISOString().slice(0, 10);
    const label =
      key === 'unknown'
        ? 'Undated'
        : date.toLocaleDateString(undefined, {
            weekday: 'long',
            month: 'long',
            day: 'numeric',
            year: 'numeric'
          });

    if (!groups.has(key)) {
      groups.set(key, { key, label, articles: [] });
    }
    groups.get(key)!.articles.push(article);
  }

  return [...groups.values()];
}

export function clamp(value: number, min: number, max: number) {
  return Math.min(max, Math.max(min, value));
}
