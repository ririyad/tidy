<script lang="ts">
  import { clamp } from '../format';
  import type { ArticleDetail, ReaderSettings } from '../types';

  let {
    article,
    settings,
    onSettings,
    onToggleRead,
    onToggleStar,
    onToggleArchive,
    onProgress
  }: {
    article: ArticleDetail | null;
    settings: ReaderSettings;
    onSettings: (next: ReaderSettings) => void;
    onToggleRead: () => void;
    onToggleStar: () => void;
    onToggleArchive: () => void;
    onProgress: (progress: number) => void;
  } = $props();

  let scroller: HTMLElement | undefined = $state();

  function updateSetting<K extends keyof ReaderSettings>(key: K, value: ReaderSettings[K]) {
    onSettings({ ...settings, [key]: value });
  }

  function onScroll() {
    if (!scroller || !article) return;
    const max = scroller.scrollHeight - scroller.clientHeight;
    const progress = max <= 0 ? 1 : clamp(scroller.scrollTop / max, 0, 1);
    onProgress(progress);
  }
</script>

<section
  class="reader-theme-{settings.theme} flex h-full min-w-0 flex-col"
  style="background: var(--reader-bg); color: var(--reader-fg);"
>
  {#if !article}
    <div class="grid flex-1 place-items-center px-8 text-center">
      <div>
        <p class="text-xs font-semibold tracking-[0.16em] uppercase" style="color: var(--reader-muted)">
          Reader
        </p>
        <h2 class="mt-3 font-[family-name:var(--font-reader)] text-3xl tracking-[-0.03em]">
          Choose something to read
        </h2>
        <p class="mx-auto mt-3 max-w-md text-sm leading-7" style="color: var(--reader-muted)">
          Use <kbd class="rounded bg-black/5 px-1.5 py-0.5 text-xs">j</kbd> /
          <kbd class="rounded bg-black/5 px-1.5 py-0.5 text-xs">k</kbd> to move,
          <kbd class="rounded bg-black/5 px-1.5 py-0.5 text-xs">o</kbd> to open,
          <kbd class="rounded bg-black/5 px-1.5 py-0.5 text-xs">s</kbd> to star.
        </p>
      </div>
    </div>
  {:else}
    <header class="flex flex-wrap items-center gap-2 border-b border-black/10 px-5 py-3">
      <button
        class="rounded-lg px-2.5 py-1.5 text-sm hover:bg-black/5"
        onclick={onToggleRead}
      >
        {article.state === 'unread' ? 'Mark read' : 'Mark unread'}
      </button>
      <button class="rounded-lg px-2.5 py-1.5 text-sm hover:bg-black/5" onclick={onToggleStar}>
        {article.starred ? 'Unstar' : 'Star'}
      </button>
      <button class="rounded-lg px-2.5 py-1.5 text-sm hover:bg-black/5" onclick={onToggleArchive}>
        {article.archived ? 'Unarchive' : 'Archive'}
      </button>
      <a
        class="rounded-lg px-2.5 py-1.5 text-sm hover:bg-black/5"
        href={article.url}
        target="_blank"
        rel="noreferrer"
      >
        Original
      </a>

      <div class="ml-auto flex flex-wrap items-center gap-2 text-xs">
        <select
          class="rounded-lg border border-black/10 bg-transparent px-2 py-1"
          value={settings.theme}
          onchange={(e) => updateSetting('theme', e.currentTarget.value)}
        >
          <option value="paper">Paper</option>
          <option value="ink">Ink</option>
          <option value="sepia">Sepia</option>
        </select>
        <select
          class="rounded-lg border border-black/10 bg-transparent px-2 py-1"
          value={settings.font}
          onchange={(e) => updateSetting('font', e.currentTarget.value)}
        >
          <option value="serif">Serif</option>
          <option value="sans">Sans</option>
        </select>
        <select
          class="rounded-lg border border-black/10 bg-transparent px-2 py-1"
          value={settings.measure}
          onchange={(e) => updateSetting('measure', e.currentTarget.value)}
        >
          <option value="narrow">Narrow</option>
          <option value="wide">Wide</option>
        </select>
        <label class="flex items-center gap-1">
          Size
          <input
            type="range"
            min="16"
            max="28"
            value={settings.font_size}
            oninput={(e) => updateSetting('font_size', Number(e.currentTarget.value))}
          />
        </label>
        <label class="flex items-center gap-1">
          Lead
          <input
            type="range"
            min="140"
            max="200"
            value={Math.round(settings.line_height * 100)}
            oninput={(e) => updateSetting('line_height', Number(e.currentTarget.value) / 100)}
          />
        </label>
      </div>
    </header>

    <div class="min-h-0 flex-1 overflow-y-auto px-6 py-8" bind:this={scroller} onscroll={onScroll}>
      <article
        class="reader-font-{settings.font} reader-measure-{settings.measure} mx-auto"
        style="font-size: {settings.font_size}px; line-height: {settings.line_height};"
      >
        <p class="text-xs font-semibold tracking-[0.14em] uppercase" style="color: var(--reader-muted)">
          {article.source_title}
          · {article.reading_time} min read
        </p>
        <h1 class="mt-3 text-[1.85em] leading-[1.15] tracking-[-0.03em]">
          {article.title}
        </h1>
        <p class="mt-3 text-sm" style="color: var(--reader-muted)">
          {#if article.author}{article.author} · {/if}
          {article.published_at
            ? new Date(article.published_at).toLocaleString()
            : new Date(article.fetched_at).toLocaleString()}
        </p>
        <div class="reader-content mt-8">
          {@html article.rendered_html}
        </div>
      </article>
    </div>
  {/if}
</section>
