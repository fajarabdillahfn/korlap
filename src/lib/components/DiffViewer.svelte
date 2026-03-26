<script lang="ts">
  let { diff }: { diff: string } = $props();

  interface DiffLine {
    text: string;
    type: 'add' | 'del' | 'hunk' | 'meta' | 'context';
  }

  const lines = $derived<DiffLine[]>(
    diff.split('\n').map((text) => {
      if (text.startsWith('+++') || text.startsWith('---')) return { text, type: 'meta' };
      if (text.startsWith('+')) return { text, type: 'add' };
      if (text.startsWith('-')) return { text, type: 'del' };
      if (text.startsWith('@@')) return { text, type: 'hunk' };
      return { text, type: 'context' };
    })
  );
</script>

<div class="diff-viewer">
  <pre class="diff-pre">{#each lines as line}<span class="line {line.type}">{line.text}
</span>{/each}</pre>
</div>

<style>
  .diff-viewer {
    overflow: auto;
    background: var(--bg-card);
    border: 1px solid var(--border);
    border-radius: 6px;
    max-height: 400px;
  }

  .diff-pre {
    margin: 0;
    padding: 8px 0;
    font-family: 'JetBrains Mono', monospace;
    font-size: 12px;
    line-height: 1.5;
    white-space: pre;
  }

  .line {
    display: block;
    padding: 0 12px;
  }

  .line.add {
    background: var(--diff-add-bg);
    color: var(--diff-add);
  }

  .line.del {
    background: var(--diff-del-bg);
    color: var(--diff-del);
  }

  .line.hunk {
    color: var(--accent);
    background: var(--bg-active);
  }

  .line.meta {
    color: var(--text-muted);
  }

  .line.context {
    color: var(--text-primary);
  }
</style>
