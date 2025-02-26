<script lang="ts">
  import { query, type DataItem } from "../prometheus";
  import { onMount } from "svelte";

  const prometheusUrl =
    (window as any).APP_CONFIG?.PROMETHEUS_URL || "http://localhost:9090";

  export let prometheusQuery: string;

  let data: DataItem;
  let error: string;

  async function fetchData() {
    try {
      // Get the Prometheus URL from the environment variable
      const items = await query(prometheusQuery, prometheusUrl);
      if (items !== undefined && items.length > 0) {
        data = items[0];
      }
    } catch (err) {
      if (err instanceof Error) {
        error = err.message;
      } else {
        error = String(err);
      }
    }
  }

  // Fetch data every 10 seconds
  onMount(() => {
    fetchData();
    const interval = setInterval(fetchData, 300);
    return () => clearInterval(interval);
  });
</script>

{#if error}
  {error}
{:else if data}
  {data.sample[1]}
{:else}
  Loading...
{/if}
