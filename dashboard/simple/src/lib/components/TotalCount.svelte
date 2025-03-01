<script lang="ts">
  import { query as promQuery, type DataItem } from "../prometheus";
  import { onMount, onDestroy } from "svelte";

  export let title: string;
  export let query: string;
  export let dataSource: { url: string; interval: number };

  let value: number;
  let error: string;
  let intervalId: number;

  $: {
    if (dataSource.interval != null && dataSource.interval > 0) {
      console.log("reactive  dataSource.interval", dataSource.interval);
      console.log("reactive intervalId", intervalId);
      if (intervalId) {
        clearInterval(intervalId);
      }
      intervalId = setInterval(fetchData, dataSource.interval);
    }
  }

  async function fetchData() {
    try {
      if (
        dataSource.url === "" ||
        dataSource.interval == null ||
        dataSource.interval === 0
      ) {
        return;
      }
      const items = await promQuery(query, dataSource.url);
      if (items !== undefined && items.length > 0) {
        const data = items[0];
        value = data.sample[1];
      }
    } catch (err) {
      if (err instanceof Error) {
        error = err.message;
      } else {
        error = String(err);
      }
    }
  }

  onDestroy(() => {
    if (intervalId) {
      clearInterval(intervalId);
    }
  });
</script>

<p class="text-xs tracking-wide text-gray-500">{title}</p>
<h3 class="text-xl sm:text-2xl font-medium text-gray-800">
  {#if error}
    {error}
  {:else if value}
    {value}
  {:else}
    Loading...
  {/if}
</h3>
