<script lang="ts">
  import { query as promQuery } from "../prometheus";
  import { onDestroy } from "svelte";

  export let title: string;
  export let query: string;
  export let dataSource: {
    url: string;
    interval: number;
    isDraw: boolean;
  };

  let value: number;
  let intervalId: number;

  $: {
    if (intervalId) {
      clearInterval(intervalId);
    }
    if (dataSource.isDraw && dataSource.interval > 0) {
      intervalId = setInterval(fetchData(), dataSource.interval);
    }
  }

  function fetchData(): () => void {
    return async () => {
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
          // TODO Display error message
          console.log(err.message);
        } else {
          // TODO Display error message
          console.log(String(err));
        }
      }
    };
  }

  onDestroy(() => {
    if (intervalId) {
      clearInterval(intervalId);
    }
  });
</script>

<p class="text-xs tracking-wide text-gray-500">{title}</p>
<h3 class="text-xl sm:text-2xl font-medium text-gray-800">
  {#if value}
    {value}
  {:else}
    Loading...
  {/if}
</h3>
