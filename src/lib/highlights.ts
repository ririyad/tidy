import type { HighlightRow } from './types';

/** Escape text for safe use inside a RegExp. */
export function escapeRegExp(value: string): string {
  return value.replace(/[.*+?^${}()|[\]\\]/g, '\\$&');
}

/**
 * Wrap TextQuote-anchored highlights inside a reader content root.
 * Clears previous `.tidy-highlight` marks first.
 */
export function applyHighlights(root: HTMLElement, highlights: HighlightRow[]) {
  unwrapHighlights(root);
  for (const highlight of highlights) {
    wrapQuote(root, highlight);
  }
}

function unwrapHighlights(root: HTMLElement) {
  const marks = [...root.querySelectorAll('mark.tidy-highlight')];
  for (const mark of marks) {
    const parent = mark.parentNode;
    if (!parent) continue;
    while (mark.firstChild) parent.insertBefore(mark.firstChild, mark);
    parent.removeChild(mark);
    parent.normalize();
  }
}

function wrapQuote(root: HTMLElement, highlight: HighlightRow) {
  const walker = document.createTreeWalker(root, NodeFilter.SHOW_TEXT);
  const nodes: Text[] = [];
  let node = walker.nextNode();
  while (node) {
    nodes.push(node as Text);
    node = walker.nextNode();
  }

  const full = nodes.map((item) => item.data).join('');
  const index = findQuoteIndex(full, highlight.text, highlight.prefix, highlight.suffix);
  if (index < 0) return;

  const end = index + highlight.text.length;
  let cursor = 0;
  for (const textNode of nodes) {
    const start = cursor;
    const stop = cursor + textNode.data.length;
    cursor = stop;
    if (stop <= index || start >= end) continue;

    const localStart = Math.max(0, index - start);
    const localEnd = Math.min(textNode.data.length, end - start);
    const range = document.createRange();
    range.setStart(textNode, localStart);
    range.setEnd(textNode, localEnd);
    const mark = document.createElement('mark');
    mark.className = 'tidy-highlight';
    mark.dataset.highlightId = highlight.id;
    if (highlight.note) mark.title = highlight.note;
    try {
      range.surroundContents(mark);
    } catch {
      // Split across element boundaries — skip this fragment.
    }
  }
}

function findQuoteIndex(haystack: string, exact: string, prefix: string, suffix: string): number {
  if (!exact) return -1;
  if (prefix || suffix) {
    const pattern = `${escapeRegExp(prefix)}${escapeRegExp(exact)}${escapeRegExp(suffix)}`;
    const match = haystack.match(new RegExp(pattern));
    if (match?.index != null) {
      return match.index + prefix.length;
    }
  }
  return haystack.indexOf(exact);
}

export type SelectionQuote = {
  text: string;
  prefix: string;
  suffix: string;
  rect: DOMRect;
};

export function readSelectionQuote(root: HTMLElement): SelectionQuote | null {
  const selection = window.getSelection();
  if (!selection || selection.isCollapsed || selection.rangeCount === 0) return null;
  const range = selection.getRangeAt(0);
  if (!root.contains(range.commonAncestorContainer)) return null;
  const text = selection.toString().replace(/\s+/g, ' ').trim();
  if (!text) return null;

  const beforeRange = range.cloneRange();
  beforeRange.selectNodeContents(root);
  beforeRange.setEnd(range.startContainer, range.startOffset);
  const afterRange = range.cloneRange();
  afterRange.selectNodeContents(root);
  afterRange.setStart(range.endContainer, range.endOffset);

  const before = beforeRange.toString().replace(/\s+/g, ' ');
  const after = afterRange.toString().replace(/\s+/g, ' ');
  return {
    text,
    prefix: before.slice(-32),
    suffix: after.slice(0, 32),
    rect: range.getBoundingClientRect()
  };
}
