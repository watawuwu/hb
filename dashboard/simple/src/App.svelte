<script lang="ts">
  import EchartsLineChart from "./lib/components/EchartsLineChart.svelte";
  import TotalCount from "./lib/components/TotalCount.svelte";
  import Header from "./lib/components/Header.svelte";
  import Sidebar from "./lib/components/Sidebar.svelte";

  // Variable to control the display/hide of the chart
  let showChart = true;

  // Toggle function
  function toggleChart() {
    showChart = !showChart;
  }

  let query200 =
    'sum(http_client_request_duration_seconds_count{"status"=~"2.+|3.+"})';
  let query400 =
    'sum(http_client_request_duration_seconds_count{"status"=~"4.+"})';
  let query500 =
    'sum(http_client_request_duration_seconds_count{"status"=~"5.+"})';
</script>

<main class="bg-gray-50">
  <Header />
  <Sidebar />

  <div class="grid grid-cols-6 gap-4 p-20">
    <div class="col-span-6 border shadow-xs rounded-xl p-4 md:p-5 bg-white">
      <dev class="flex justify-end">
        <button
          on:click={toggleChart}
          type="button"
          class="col-span-1 py-3 px-4 inline-flex items-center gap-x-2 text-sm font-medium rounded-lg border border-transparent bg-blue-600 text-white hover:bg-blue-700 focus:outline-hidden focus:bg-blue-700 disabled:opacity-50 disabled:pointer-events-none"
        >
          {showChart ? "Hide Chart" : "Show Chart"}
        </button>
      </dev>
    </div>

    <div class="col-span-2 border shadow-xs rounded-xl p-4 md:p-5 bg-white">
      <p class="text-xs tracking-wide text-gray-500">Status 2xx/3xx</p>
      <h3 class="text-xl sm:text-2xl font-medium text-gray-800">
        <TotalCount prometheusQuery={query200} />
      </h3>
    </div>

    <div class="col-span-2 border shadow-xs rounded-xl p-4 md:p-5 bg-white">
      <p class="text-xs tracking-wide text-gray-500">Status 4xx</p>
      <h3 class="text-xl sm:text-2xl font-medium text-gray-800">
        <TotalCount prometheusQuery={query400} />
      </h3>
    </div>

    <div class="col-span-2 border shadow-xs rounded-xl p-4 md:p-5 bg-white">
      <p class="text-xs tracking-wide text-gray-500">Status 5xx</p>
      <h3 class="text-xl sm:text-2xl font-medium text-gray-800">
        <TotalCount prometheusQuery={query500} />
      </h3>
    </div>

    {#if showChart}
      <div class="col-span-6 border shadow-xs rounded-xl md:p-5 bg-white">
        <EchartsLineChart
          title="Request per second"
          yAxisName="RPS"
          prometheusQuery="sum by(method, path, status) (rate(http_client_request_duration_seconds_count[10s]))"
        />
      </div>

      <div class="col-span-6 border shadow-xs rounded-xl md:p-5 bg-white">
        <EchartsLineChart
          title="Request latency(99 percentile)"
          yAxisName="Latency"
          prometheusQuery="histogram_quantile(0.99, sum by (method, path, status, le) (rate(http_client_request_duration_seconds_bucket[10s])))"
        />
      </div>
    {/if}
  </div>
</main>

<style>
</style>
