<script lang="ts">
  let {
    open,
    step,
    appVersion,
    onAddSource,
    onDismiss,
    onCloseHelp
  }: {
    open: boolean;
    step: 'welcome' | 'help' | null;
    appVersion: string;
    onAddSource: () => void;
    onDismiss: () => void;
    onCloseHelp: () => void;
  } = $props();
</script>

{#if open && step === 'welcome'}
  <div class="fixed inset-0 z-50 grid place-items-center bg-[rgb(19_32_51/45%)] p-4">
    <div class="w-full max-w-lg rounded-3xl border border-[var(--line)] bg-[var(--paper)] p-6 shadow-2xl">
      <p class="text-xs font-semibold tracking-[0.16em] text-[var(--accent)] uppercase">Welcome</p>
      <h2 class="mt-2 font-[family-name:var(--font-reader)] text-3xl tracking-[-0.03em]">
        Your vault is ready
      </h2>
      <ol class="mt-5 space-y-3 text-sm leading-6 text-[var(--ink-soft)]">
        <li><span class="font-semibold text-[var(--ink)]">1.</span> Add a blog URL prefix</li>
        <li><span class="font-semibold text-[var(--ink)]">2.</span> Let Tidy fetch recent posts</li>
        <li><span class="font-semibold text-[var(--ink)]">3.</span> Read, search, highlight, and save views</li>
      </ol>
      <p class="mt-4 text-xs text-[var(--ink-soft)]">Tidy {appVersion} · local-first · Obsidian-friendly markdown</p>
      <div class="mt-6 flex justify-end gap-2">
        <button
          class="rounded-xl px-4 py-2.5 text-sm text-[var(--ink-soft)] hover:bg-white"
          onclick={onDismiss}
        >
          Browse empty vault
        </button>
        <button
          class="rounded-xl bg-[var(--ink)] px-4 py-2.5 text-sm font-semibold text-[var(--paper)]"
          onclick={onAddSource}
        >
          Add a source
        </button>
      </div>
    </div>
  </div>
{/if}

{#if open && step === 'help'}
  <div class="fixed inset-0 z-50 grid place-items-center bg-[rgb(19_32_51/45%)] p-4">
    <div class="w-full max-w-md rounded-3xl border border-[var(--line)] bg-[var(--paper)] p-6 shadow-2xl">
      <h2 class="font-[family-name:var(--font-reader)] text-2xl tracking-[-0.03em]">Shortcuts</h2>
      <dl class="mt-4 space-y-2 text-sm">
        <div class="flex justify-between gap-4"><dt class="text-[var(--ink-soft)]">Move</dt><dd><kbd>j</kbd> / <kbd>k</kbd></dd></div>
        <div class="flex justify-between gap-4"><dt class="text-[var(--ink-soft)]">Open</dt><dd><kbd>o</kbd></dd></div>
        <div class="flex justify-between gap-4"><dt class="text-[var(--ink-soft)]">Read / unread</dt><dd><kbd>u</kbd></dd></div>
        <div class="flex justify-between gap-4"><dt class="text-[var(--ink-soft)]">Star</dt><dd><kbd>s</kbd></dd></div>
        <div class="flex justify-between gap-4"><dt class="text-[var(--ink-soft)]">Archive</dt><dd><kbd>e</kbd></dd></div>
        <div class="flex justify-between gap-4"><dt class="text-[var(--ink-soft)]">Refresh</dt><dd><kbd>r</kbd></dd></div>
        <div class="flex justify-between gap-4"><dt class="text-[var(--ink-soft)]">Search</dt><dd><kbd>/</kbd></dd></div>
        <div class="flex justify-between gap-4"><dt class="text-[var(--ink-soft)]">Inbox</dt><dd><kbd>g</kbd> then <kbd>f</kbd></dd></div>
        <div class="flex justify-between gap-4"><dt class="text-[var(--ink-soft)]">Help</dt><dd><kbd>?</kbd></dd></div>
      </dl>
      <p class="mt-4 text-xs text-[var(--ink-soft)]">Select text in the reader to highlight.</p>
      <div class="mt-5 flex justify-end">
        <button
          class="rounded-xl bg-[var(--ink)] px-4 py-2.5 text-sm font-semibold text-[var(--paper)]"
          onclick={onCloseHelp}
        >
          Close
        </button>
      </div>
    </div>
  </div>
{/if}
