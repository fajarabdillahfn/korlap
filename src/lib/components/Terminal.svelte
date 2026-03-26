<script lang="ts">
  import { onMount, onDestroy } from 'svelte';
  import { invoke } from '@tauri-apps/api/core';
  import { listen } from '@tauri-apps/api/event';

  let { taskId, terminalId }: { taskId: string; terminalId: string } = $props();

  let container = $state<HTMLDivElement | undefined>(undefined);
  let terminal: import('@xterm/xterm').Terminal | undefined;
  let unlisten: (() => void) | undefined;
  let unlistenExit: (() => void) | undefined;

  /** Decode base64 → Uint8Array for binary-safe terminal writes. */
  function decodeBase64(b64: string): Uint8Array {
    const bin = atob(b64);
    const bytes = new Uint8Array(bin.length);
    for (let i = 0; i < bin.length; i++) bytes[i] = bin.charCodeAt(i);
    return bytes;
  }

  /** Only fit when the container has non-zero dimensions (not hidden). */
  function safeFit(fitAddon: import('@xterm/addon-fit').FitAddon) {
    if (container && container.offsetWidth > 0 && container.offsetHeight > 0) {
      fitAddon.fit();
    }
  }

  onMount(async () => {
    const { Terminal } = await import('@xterm/xterm');
    const { FitAddon } = await import('@xterm/addon-fit');

    terminal = new Terminal({
      theme: {
        background: '#0f0d0a',
        foreground: '#f0e6d0',
        cursor: '#d48f3a',
        selectionBackground: '#2d2518',
      },
      fontFamily: '"JetBrains Mono", "Fira Code", monospace',
      fontSize: 13,
      cursorBlink: true,
    });

    const fitAddon = new FitAddon();
    terminal.loadAddon(fitAddon);

    if (container) {
      terminal.open(container);
      safeFit(fitAddon);
    }

    terminal.onData((data) => {
      invoke('pty_write', { terminal_id: terminalId, data }).catch(() => {});
    });

    // Listen for output (base64-encoded bytes from backend)
    unlisten = await listen<{ terminal_id: string; data: string }>('terminal:stdout', (event) => {
      if (event.payload.terminal_id === terminalId) {
        terminal?.write(decodeBase64(event.payload.data));
      }
    });

    unlistenExit = await listen<{ terminal_id: string; code: number | null }>('terminal:exit', (event) => {
      if (event.payload.terminal_id === terminalId) {
        const codeStr = event.payload.code != null ? String(event.payload.code) : '?';
        terminal?.write(`\r\n\x1b[90m[Process exited with code ${codeStr}]\x1b[0m\r\n`);
      }
    });

    // Start the PTY
    await invoke('pty_create', { task_id: taskId, terminal_id: terminalId }).catch((e) => {
      terminal?.write(`\r\n\x1b[31mFailed to start terminal: ${e}\x1b[0m\r\n`);
    });

    // Handle resize — guard against hidden-container fit calls
    const resizeObserver = new ResizeObserver(() => safeFit(fitAddon));
    if (container) resizeObserver.observe(container);

    return () => resizeObserver.disconnect();
  });

  onDestroy(() => {
    unlisten?.();
    unlistenExit?.();
    // Fire-and-forget: IPC message is dispatched; Svelte onDestroy is synchronous.
    invoke('pty_kill', { terminal_id: terminalId }).catch(() => {});
    terminal?.dispose();
  });
</script>

<div class="terminal-container" bind:this={container}></div>

<style>
  .terminal-container {
    width: 100%;
    height: 100%;
    overflow: hidden;
  }

  :global(.terminal-container .xterm) {
    height: 100%;
    padding: 8px;
  }
</style>
