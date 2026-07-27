<script lang="ts">
  import { tick } from 'svelte';
  import { clamp } from '../format';
  import { applyHighlights, readSelectionQuote, type SelectionQuote } from '../highlights';
  import type { ArticleDetail, HighlightRow, ReaderSettings } from '../types';

  let {
    article,
    settings,
    onSettings,
    onToggleRead,
    onToggleStar,
    onToggleArchive,
    onProgress,
    onAddHighlight,
    onUpdateHighlightNote,
    onDeleteHighlight
  }: {
    article: ArticleDetail | null;
    settings: ReaderSettings;
    onSettings: (next: ReaderSettings) => void;
    onToggleRead: () => void;
    onToggleStar: () => void;
    onToggleArchive: () => void;
    onProgress: (progress: number) => void;
    onAddHighlight: (payload: {
      text: string;
      note?: string | null;
      prefix?: string | null;
      suffix?: string | null;
    }) => Promise<void>;
    onUpdateHighlightNote: (id: string, note: string | null) => Promise<void>;
    onDeleteHighlight: (id: string) => Promise<void>;
  } = $props();

  let scroller: HTMLElement | undefined = $state();
  let contentRoot: HTMLElement | undefined = $state();
  let pending = $state<SelectionQuote | null>(null);
  let noteDraft = $state('');
  let busy = $state(false);

  $effect(() => {
    const highlights = article?.highlights ?? [];
    void tick().then(() => {
      if (contentRoot) applyHighlights(contentRoot, highlights);
    });
  });

  function updateSetting<K extends keyof ReaderSettings>(key: K, value: ReaderSettings[K]) {
    onSettings({ ...settings, [key]: value });
  }

  function onScroll() {
    if (!scroller || !article) return;
    const max = scroller.scrollHeight - scroller.clientHeight;
    const progress = max <= 0 ? 1 : clamp(scroller.scrollTop / max, 0, 1);
    onProgress(progress);
  }

  function onMouseUp() {
    if (!contentRoot || !article) return;
    const quote = readSelectionQuote(contentRoot);
    if (!quote) return;
    pending = quote;
    noteDraft = '';
  }

  async function saveHighlight() {
    if (!pending || !article || busy) return;
    busy = true;
    try {
      await onAddHighlight({
        text: pending.text,
        note: noteDraft.trim() || null,
        prefix: pending.prefix,
        suffix: pending.suffix
      });
      pending = null;
      noteDraft = '';
      window.getSelection()?.removeAllRanges();
    } finally {
      busy = false;
    }
  }

  function cancelPending() {
    pending = null;
    noteDraft = '';
    window.getSelection()?.removeAllRanges();
  }

  async function editNote(highlight: HighlightRow) {
    const next = window.prompt('Note for highlight', highlight.note ?? '');
    if (next === null) return;
    await onUpdateHighlightNote(highlight.id, next.trim() || null);
  }
</script>

<section
  class="reader-theme-{settings.theme} relative flex h-full min-w-0 flex-col"
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
          Select text to highlight.
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
        <div
          class="reader-content mt-8"
          role="region"
          aria-label="Article body"
          bind:this={contentRoot}
          onmouseup={onMouseUp}
        >
          {@html article.rendered_html}
        </div>

        {#if article.highlights.length > 0}
          <section class="mt-12 border-t border-black/10 pt-6">
            <p
              class="text-xs font-semibold tracking-[0.14em] uppercase"
              style="color: var(--reader-muted)"
            >
              Highlights
            </p>
            <ul class="mt-4 space-y-4">
              {#each article.highlights as highlight}
                <li class="rounded-2xl border border-black/10 bg-black/[0.03] px-4 py-3">
                  <blockquote class="text-[0.95em] leading-relaxed italic">
                    “{highlight.text}”
                  </blockquote>
                  {#if highlight.note}
                    <p class="mt-2 text-sm" style="color: var(--reader-muted)">{highlight.note}</p>
                  {/if}
                  <div class="mt-3 flex gap-3 text-xs">
                    <button class="hover:underline" onclick={() => editNote(highlight)}>
                      {highlight.note ? 'Edit note' : 'Add note'}
                    </button>
                    <button class="hover:underline" onclick={() => onDeleteHighlight(highlight.id)}>
                      Remove
                    </button>
                  </div>
                </li>
              {/each}
            </ul>
          </section>
        {/if}
      </article>
    </div>

    {#if pending}
      <div
        class="fixed z-30 w-72 rounded-2xl border border-black/10 bg-white p-3 shadow-xl"
        style="left: {Math.min(pending.rect.left, window.innerWidth - 300)}px; top: {Math.min(
          pending.rect.bottom + 8,
          window.innerHeight - 180
        )}px;"
      >
        <p class="line-clamp-3 text-sm italic text-[var(--ink-soft)]">“{pending.text}”</p>
        <textarea
          class="mt-2 w-full rounded-xl border border-black/10 px-2 py-1.5 text-sm"
          rows="2"
          placeholder="Optional note"
          bind:value={noteDraft}
        ></textarea>
        <div class="mt-2 flex justify-end gap-2">
          <button class="rounded-lg px-2.5 py-1.5 text-sm hover:bg-black/5" onclick={cancelPending}>
            Cancel
          </button>
          <button
            class="rounded-lg bg-[var(--ink)] px-2.5 py-1.5 text-sm font-semibold text-[var(--paper)] disabled:opacity-60"
            onclick={saveHighlight}
            disabled={busy}
          >
            Highlight
          </button>
        </div>
      </div>
    {/if}
  {/if}
</section>
