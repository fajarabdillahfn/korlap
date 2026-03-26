<script lang="ts">
  import Titlebar from '$lib/components/Titlebar.svelte';
  import Sidebar from '$lib/components/Sidebar.svelte';
  import Toast from '$lib/components/Toast.svelte';
  import '../app.css';

  let { children } = $props();
</script>

<Toast />
<div class="app-shell">
  <Titlebar />
  <div class="app-body">
    <Sidebar />
    <main class="app-main">
      {@render children()}
    </main>
  </div>
</div>

<style>
  .app-shell {
    display: flex;
    flex-direction: column;
    height: 100vh;
    overflow: hidden;
    background: var(--bg-base);
  }

  .app-body {
    display: flex;
    flex: 1;
    overflow: hidden;
  }

  .app-main {
    flex: 1;
    overflow: auto;
  }
</style>
