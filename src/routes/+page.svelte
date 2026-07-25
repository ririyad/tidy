<script lang="ts">
  import { invoke } from '@tauri-apps/api/core';

  type VaultSummary = {
    path: string;
    database_path: string;
    created: boolean;
  };

  let vault = $state<VaultSummary | null>(null);
  let choosing = $state(false);
  let error = $state('');

  async function chooseVault() {
    choosing = true;
    error = '';

    try {
      vault = await invoke<VaultSummary | null>('select_vault');
    } catch (cause) {
      error = cause instanceof Error ? cause.message : String(cause);
    } finally {
      choosing = false;
    }
  }
</script>

<svelte:head>
  <title>Tidy — Your information, thoughtfully gathered</title>
</svelte:head>

<main class="min-h-screen px-6 py-8 sm:px-10 sm:py-12">
  <div class="mx-auto flex min-h-[calc(100vh-6rem)] max-w-6xl flex-col">
    <header class="flex items-center justify-between">
      <div class="flex items-center gap-3">
        <div class="grid size-10 place-items-center rounded-full bg-[#2d3224] text-[#f4f1e8]">
          <span class="text-lg font-semibold">T</span>
        </div>
        <span class="text-sm font-semibold tracking-[0.18em] uppercase">Tidy</span>
      </div>
      <span class="rounded-full border border-[#cecbbf] bg-white/45 px-3 py-1 text-xs text-[#696b61]">
        Local-first
      </span>
    </header>

    <section class="grid flex-1 items-center gap-14 py-16 lg:grid-cols-[1.1fr_0.9fr]">
      <div>
        <p class="mb-5 text-sm font-semibold tracking-[0.16em] text-[#69764c] uppercase">
          Your information feed
        </p>
        <h1 class="max-w-3xl font-serif text-5xl leading-[1.02] tracking-[-0.045em] sm:text-7xl">
          Read what matters.<br />Keep it yours.
        </h1>
        <p class="mt-7 max-w-xl text-lg leading-8 text-[#64665e]">
          Tidy gathers writing from the sites you choose, stores it as portable Markdown, and
          gives every article one calm place to read.
        </p>

        <div class="mt-10">
          <button
            class="inline-flex min-h-12 items-center gap-3 rounded-full bg-[#2d3224] px-6 py-3 font-semibold text-[#faf8f1] shadow-[0_10px_30px_rgb(45_50_36/18%)] transition hover:-translate-y-0.5 hover:bg-[#3a4130] disabled:cursor-wait disabled:opacity-70"
            onclick={chooseVault}
            disabled={choosing}
          >
            {choosing ? 'Opening…' : vault ? 'Choose another vault' : 'Choose your vault'}
            <span aria-hidden="true">→</span>
          </button>
          <p class="mt-3 text-sm text-[#7b7d74]">
            Select an empty folder or an existing Tidy vault. Your files stay on your machine.
          </p>
        </div>

        {#if error}
          <p class="mt-5 max-w-xl rounded-xl border border-red-200 bg-red-50 px-4 py-3 text-sm text-red-800">
            {error}
          </p>
        {/if}

        {#if vault}
          <div class="mt-6 max-w-xl rounded-2xl border border-[#c8ceb0] bg-[#edf0dc] p-5">
            <p class="font-semibold text-[#3f482d]">
              {vault.created ? 'Your vault is ready.' : 'Vault opened.'}
            </p>
            <p class="mt-1 truncate font-mono text-xs text-[#687052]" title={vault.path}>
              {vault.path}
            </p>
          </div>
        {/if}
      </div>

      <div class="relative hidden min-h-[34rem] lg:block" aria-hidden="true">
        <div class="absolute inset-8 rotate-3 rounded-[2rem] border border-[#d0ccbf] bg-[#e9e4d8]"></div>
        <article class="absolute inset-0 -rotate-2 overflow-hidden rounded-[2rem] border border-[#d8d4c8] bg-[#fcfbf7] p-9 shadow-[0_30px_70px_rgb(55_52_40/12%)]">
          <div class="flex items-center justify-between text-xs font-medium text-[#838177]">
            <span>THE MARGINALIAN</span>
            <span>12 MIN READ</span>
          </div>
          <div class="mt-12 h-3 w-24 rounded-full bg-[#dce2c6]"></div>
          <h2 class="mt-5 font-serif text-4xl leading-tight tracking-[-0.03em]">
            The quiet art of paying attention
          </h2>
          <p class="mt-4 text-sm leading-6 text-[#76766f]">Maria Popova · Today</p>
          <div class="mt-10 space-y-3">
            <div class="h-2 rounded-full bg-[#e5e2da]"></div>
            <div class="h-2 rounded-full bg-[#e5e2da]"></div>
            <div class="h-2 w-5/6 rounded-full bg-[#e5e2da]"></div>
          </div>
          <blockquote class="mt-10 border-l-2 border-[#9eaa73] pl-6 font-serif text-2xl leading-9 text-[#4d5046]">
            “Attention is the rarest and purest form of generosity.”
          </blockquote>
          <div class="mt-10 space-y-3">
            <div class="h-2 rounded-full bg-[#e5e2da]"></div>
            <div class="h-2 w-11/12 rounded-full bg-[#e5e2da]"></div>
            <div class="h-2 w-3/4 rounded-full bg-[#e5e2da]"></div>
          </div>
        </article>
      </div>
    </section>

    <footer class="flex items-center justify-between border-t border-[#d8d3c5] pt-5 text-xs text-[#85867d]">
      <span>Private by default</span>
      <span>Markdown + SQLite</span>
    </footer>
  </div>
</main>
