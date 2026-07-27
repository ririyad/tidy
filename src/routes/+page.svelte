<script lang="ts">
  import { listen } from '@tauri-apps/api/event';
  import { onMount } from 'svelte';
  import { api } from '$lib/api';
  import AddSourceModal from '$lib/components/AddSourceModal.svelte';
  import FeedList from '$lib/components/FeedList.svelte';
  import Reader from '$lib/components/Reader.svelte';
  import Sidebar from '$lib/components/Sidebar.svelte';
  import type {
    ArticleDetail,
    ArticleListItem,
    FeedFilter,
    FetchProgress,
    FetchRunRow,
    ReaderSettings,
    SmartViewRow,
    SourceRow,
    TagCount,
    VaultSummary
  } from '$lib/types';

  let vault = $state<VaultSummary | null>(null);
  let choosing = $state(false);
  let error = $state('');
  let sources = $state<SourceRow[]>([]);
  let articles = $state<ArticleListItem[]>([]);
  let selectedId = $state<number | null>(null);
  let selectedSourceId = $state<number | null>(null);
  let article = $state<ArticleDetail | null>(null);
  let filter = $state<FeedFilter>('inbox');
  let searchQuery = $state('');
  let selectedTag = $state<string | null>(null);
  let activeSmartViewId = $state<string | null>(null);
  let tags = $state<TagCount[]>([]);
  let smartViews = $state<SmartViewRow[]>([]);
  let searchInput = $state<HTMLInputElement | null>(null);
  let settings = $state<ReaderSettings>({
    theme: 'paper',
    font: 'serif',
    font_size: 20,
    line_height: 1.7,
    measure: 'narrow'
  });
  let showAdd = $state(false);
  let fetching = $state(false);
  let progressMessage = $state('');
  let fetchRuns = $state<FetchRunRow[]>([]);
  let settingsTimer: ReturnType<typeof setTimeout> | null = null;
  let progressTimer: ReturnType<typeof setTimeout> | null = null;
  let stateTimer: ReturnType<typeof setTimeout> | null = null;
  let scheduleTimer: ReturnType<typeof setInterval> | null = null;
  let searchTimer: ReturnType<typeof setTimeout> | null = null;

  onMount(() => {
    let unlisten: (() => void) | undefined;
    listen<FetchProgress>('fetch-progress', (event) => {
      progressMessage = event.payload.message;
    }).then((fn) => {
      unlisten = fn;
    });
    return () => {
      unlisten?.();
      if (scheduleTimer) clearInterval(scheduleTimer);
    };
  });

  async function chooseVault() {
    choosing = true;
    error = '';
    try {
      const next = await api.selectVault();
      if (next) {
        vault = next;
        await bootstrapVault();
      }
    } catch (cause) {
      error = cause instanceof Error ? cause.message : String(cause);
    } finally {
      choosing = false;
    }
  }

  async function bootstrapVault() {
    sources = await api.listSources();
    settings = await api.getReaderSettings();
    tags = await api.listTags();
    smartViews = await api.listSmartViews();
    await reloadArticles();
    await reloadFetchRuns();
    startScheduleTicker();
    void catchUp();
  }

  async function reloadTags() {
    tags = await api.listTags();
  }

  async function reloadSmartViews() {
    smartViews = await api.listSmartViews();
  }

  function startScheduleTicker() {
    if (scheduleTimer) clearInterval(scheduleTimer);
    // Soft in-app tick: catch up any due sources about once a minute while open.
    scheduleTimer = setInterval(() => {
      if (!vault || fetching) return;
      void catchUp({ quiet: true });
    }, 60_000);
  }

  async function reloadFetchRuns() {
    fetchRuns = await api.listFetchRuns(selectedSourceId, 8);
  }

  async function catchUp(options?: { quiet?: boolean }) {
    if (fetching) return;
    const due = await api.listDueSources();
    if (due.length === 0) return;
    fetching = true;
    if (!options?.quiet) error = '';
    try {
      progressMessage = `Catching up ${due.length} source${due.length === 1 ? '' : 's'}…`;
      await api.catchUpDueSources();
      sources = await api.listSources();
      await reloadArticles();
      await reloadTags();
      await reloadFetchRuns();
    } catch (cause) {
      if (!options?.quiet) {
        error = cause instanceof Error ? cause.message : String(cause);
      }
    } finally {
      fetching = false;
      progressMessage = '';
    }
  }

  async function reloadArticles() {
    articles = await api.queryArticles({
      filter,
      source_id: selectedSourceId,
      tag: selectedTag,
      search: searchQuery.trim() || null
    });
    if (selectedId && !articles.some((item) => item.id === selectedId)) {
      selectedId = articles[0]?.id ?? null;
    }
    if (selectedId) {
      article = await api.getArticle(selectedId);
    } else {
      article = null;
    }
  }

  function scheduleSearchReload(value: string) {
    searchQuery = value;
    activeSmartViewId = null;
    if (searchTimer) clearTimeout(searchTimer);
    searchTimer = setTimeout(() => {
      void reloadArticles();
    }, 220);
  }

  function clearQueryContext() {
    activeSmartViewId = null;
  }

  async function saveSmartView() {
    const name = window.prompt('Save smart view as…');
    if (!name?.trim()) return;
    const saved = await api.saveSmartView({
      name: name.trim(),
      query: {
        filter,
        tag: selectedTag,
        query: searchQuery.trim() || null,
        source_id: selectedSourceId
      }
    });
    smartViews = await api.listSmartViews();
    activeSmartViewId = saved.id;
  }

  async function deleteSmartView(id: string) {
    await api.deleteSmartView(id);
    if (activeSmartViewId === id) activeSmartViewId = null;
    smartViews = await api.listSmartViews();
  }

  function applySmartView(view: SmartViewRow | null) {
    if (!view) {
      activeSmartViewId = null;
      return;
    }
    activeSmartViewId = view.id;
    filter = view.query.filter ?? 'inbox';
    selectedTag = view.query.tag ?? null;
    searchQuery = view.query.query ?? '';
    selectedSourceId = view.query.source_id ?? null;
    void reloadArticles();
    void reloadFetchRuns();
  }

  async function openArticle(id: number) {
    selectedId = id;
    article = await api.getArticle(id);
    if (article && article.state === 'unread') {
      scheduleState({ id, state: 'read' });
    }
  }

  function scheduleState(patch: {
    id: number;
    state?: string;
    starred?: boolean;
    archived?: boolean;
  }) {
    if (stateTimer) clearTimeout(stateTimer);
    stateTimer = setTimeout(async () => {
      article = await api.setArticleState(patch);
      sources = await api.listSources();
      await reloadArticles();
    }, 180);
  }

  function scheduleSettings(next: ReaderSettings) {
    settings = next;
    if (settingsTimer) clearTimeout(settingsTimer);
    settingsTimer = setTimeout(async () => {
      settings = await api.setReaderSettings(next);
    }, 250);
  }

  function scheduleProgress(progress: number) {
    if (!article) return;
    if (progressTimer) clearTimeout(progressTimer);
    progressTimer = setTimeout(() => {
      void api.setArticleProgress(article!.id, progress);
    }, 400);
  }

  async function handleAddSource(payload: {
    url_prefix: string;
    title?: string;
    backfill: 'recent' | 'full';
    recent_limit: number;
    interval_minutes: number;
  }) {
    fetching = true;
    error = '';
    try {
      await api.addSource(payload);
      showAdd = false;
      sources = await api.listSources();
      filter = 'inbox';
      await reloadArticles();
      await reloadFetchRuns();
    } catch (cause) {
      error = cause instanceof Error ? cause.message : String(cause);
    } finally {
      fetching = false;
      progressMessage = '';
    }
  }

  async function refresh() {
    fetching = true;
    error = '';
    try {
      if (selectedSourceId) {
        await api.refreshSource(selectedSourceId);
      } else {
        for (const source of sources.filter((item) => item.enabled)) {
          await api.refreshSource(source.id);
        }
      }
      sources = await api.listSources();
      await reloadArticles();
      await reloadTags();
      await reloadFetchRuns();
    } catch (cause) {
      error = cause instanceof Error ? cause.message : String(cause);
    } finally {
      fetching = false;
      progressMessage = '';
    }
  }

  async function removeSource(id: number) {
    await api.removeSource(id);
    if (selectedSourceId === id) selectedSourceId = null;
    sources = await api.listSources();
    await reloadArticles();
    await reloadFetchRuns();
  }

  async function toggleSourceEnabled(id: number, enabled: boolean) {
    await api.updateSourceSchedule({ sourceId: id, enabled });
    sources = await api.listSources();
  }

  async function changeSourceInterval(id: number, minutes: number) {
    await api.updateSourceSchedule({ sourceId: id, intervalMinutes: minutes });
    sources = await api.listSources();
  }

  function moveSelection(delta: number) {
    if (articles.length === 0) return;
    const index = articles.findIndex((item) => item.id === selectedId);
    const nextIndex =
      index < 0 ? 0 : Math.min(articles.length - 1, Math.max(0, index + delta));
    void openArticle(articles[nextIndex].id);
  }

  let awaitingGo = $state(false);

  function onKeydown(event: KeyboardEvent) {
    if (!vault) return;
    const target = event.target as HTMLElement | null;
    if (target && ['INPUT', 'TEXTAREA', 'SELECT'].includes(target.tagName)) return;

    if (awaitingGo) {
      awaitingGo = false;
      if (event.key === 'f') {
        event.preventDefault();
        filter = 'inbox';
        selectedSourceId = null;
        selectedTag = null;
        searchQuery = '';
        clearQueryContext();
        void reloadArticles();
      } else if (event.key === 'r') {
        event.preventDefault();
        filter = 'starred';
        selectedTag = null;
        searchQuery = '';
        clearQueryContext();
        void reloadArticles();
      }
      return;
    }

    switch (event.key) {
      case 'j':
        event.preventDefault();
        moveSelection(1);
        break;
      case 'k':
        event.preventDefault();
        moveSelection(-1);
        break;
      case 'o':
        if (selectedId) void openArticle(selectedId);
        break;
      case 'u':
        if (article) {
          scheduleState({
            id: article.id,
            state: article.state === 'unread' ? 'read' : 'unread'
          });
        }
        break;
      case 's':
        if (article) scheduleState({ id: article.id, starred: !article.starred });
        break;
      case 'e':
        if (article) scheduleState({ id: article.id, archived: !article.archived });
        break;
      case 'r':
        void refresh();
        break;
      case 'g':
        awaitingGo = true;
        break;
      case '/':
        event.preventDefault();
        searchInput?.focus();
        break;
    }
  }
</script>

<svelte:window onkeydown={onKeydown} />

<svelte:head>
  <title>Tidy — Information feed</title>
</svelte:head>

{#if !vault}
  <main class="min-h-screen px-6 py-10 sm:px-10">
    <div class="mx-auto flex min-h-[calc(100vh-5rem)] max-w-6xl flex-col">
      <header class="flex items-center justify-between">
        <div class="flex items-center gap-3">
          <div class="grid size-10 place-items-center rounded-full bg-[var(--ink)] text-[var(--paper)]">
            <span class="text-lg font-semibold">T</span>
          </div>
          <span class="text-sm font-semibold tracking-[0.18em] uppercase">Tidy</span>
        </div>
        <span class="text-xs text-[var(--ink-soft)]">Local-first</span>
      </header>

      <section class="grid flex-1 items-center gap-12 py-16 lg:grid-cols-[1.05fr_0.95fr]">
        <div>
          <p class="mb-4 text-sm font-semibold tracking-[0.16em] text-[var(--accent)] uppercase">
            Your information feed
          </p>
          <h1
            class="max-w-3xl font-[family-name:var(--font-reader)] text-5xl leading-[1.02] tracking-[-0.045em] sm:text-7xl"
          >
            Read what matters.<br />Keep it yours.
          </h1>
          <p class="mt-6 max-w-xl text-lg leading-8 text-[var(--ink-soft)]">
            Choose a vault folder. Tidy fetches the blogs you care about into portable Markdown
            and a calm reading view.
          </p>
          <button
            class="mt-9 inline-flex min-h-12 items-center gap-3 rounded-full bg-[var(--ink)] px-6 py-3 font-semibold text-[var(--paper)]"
            onclick={chooseVault}
            disabled={choosing}
          >
            {choosing ? 'Opening…' : 'Choose your vault'}
            <span aria-hidden="true">→</span>
          </button>
          {#if error}
            <p class="mt-5 max-w-xl rounded-xl border border-red-200 bg-red-50 px-4 py-3 text-sm text-red-800">
              {error}
            </p>
          {/if}
        </div>
        <div class="relative hidden min-h-[28rem] lg:block" aria-hidden="true">
          <div class="absolute inset-0 rotate-2 rounded-[2rem] bg-[var(--paper-deep)]"></div>
          <div
            class="absolute inset-0 -rotate-2 overflow-hidden rounded-[2rem] border border-[var(--line)] bg-white/80 p-8 shadow-[0_30px_70px_rgb(19_32_51/10%)]"
          >
            <p class="text-xs font-semibold tracking-[0.14em] text-[var(--ink-soft)] uppercase">
              Inbox
            </p>
            <h2 class="mt-8 font-[family-name:var(--font-reader)] text-4xl tracking-[-0.03em]">
              The quiet craft of attention
            </h2>
            <p class="mt-4 text-sm text-[var(--ink-soft)]">Today · 11 min</p>
            <div class="mt-10 space-y-3">
              <div class="h-2 rounded-full bg-[var(--paper-deep)]"></div>
              <div class="h-2 w-5/6 rounded-full bg-[var(--paper-deep)]"></div>
              <div class="h-2 w-4/5 rounded-full bg-[var(--paper-deep)]"></div>
            </div>
          </div>
        </div>
      </section>
    </div>
  </main>
{:else}
  <div class="app-shell">
    <Sidebar
      {sources}
      {filter}
      {selectedSourceId}
      {selectedTag}
      {activeSmartViewId}
      {tags}
      {smartViews}
      {fetching}
      {fetchRuns}
      onFilter={(next: FeedFilter) => {
        filter = next;
        clearQueryContext();
        void reloadArticles();
      }}
      onSelectSource={(id: number | null) => {
        selectedSourceId = id;
        clearQueryContext();
        void reloadArticles();
        void reloadFetchRuns();
      }}
      onSelectTag={(tag: string | null) => {
        selectedTag = tag;
        clearQueryContext();
        void reloadArticles();
      }}
      onSelectSmartView={applySmartView}
      onSaveSmartView={() => void saveSmartView()}
      onDeleteSmartView={(id: string) => void deleteSmartView(id)}
      onAddSource={() => (showAdd = true)}
      onRefresh={refresh}
      onRemoveSource={removeSource}
      onChangeVault={chooseVault}
      onToggleEnabled={toggleSourceEnabled}
      onChangeInterval={changeSourceInterval}
    />
    <FeedList
      {articles}
      {selectedId}
      {progressMessage}
      {searchQuery}
      bind:searchInput
      onSelect={openArticle}
      onSearchChange={scheduleSearchReload}
    />
    <Reader
      {article}
      {settings}
      onSettings={scheduleSettings}
      onToggleRead={() =>
        article &&
        scheduleState({
          id: article.id,
          state: article.state === 'unread' ? 'read' : 'unread'
        })}
      onToggleStar={() => article && scheduleState({ id: article.id, starred: !article.starred })}
      onToggleArchive={() =>
        article && scheduleState({ id: article.id, archived: !article.archived })}
      onProgress={scheduleProgress}
    />
  </div>

  {#if error}
    <div class="fixed bottom-4 left-1/2 z-40 max-w-xl -translate-x-1/2 rounded-xl border border-red-200 bg-red-50 px-4 py-3 text-sm text-red-800 shadow-lg">
      {error}
    </div>
  {/if}
{/if}

<AddSourceModal
  open={showAdd}
  busy={fetching}
  onClose={() => (showAdd = false)}
  onSubmit={handleAddSource}
/>
