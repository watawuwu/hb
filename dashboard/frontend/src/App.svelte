<script lang="ts">
  import EchartsLineChart from "./lib/components/EchartsLineChart.svelte";
  import TotalCount from "./lib/components/TotalCount.svelte";
  import Header from "./lib/components/Header.svelte";

  const url = (window as any).dashboardConfig?.datasourceUrl || "";

  let dataSource = {
    url,
    interval: 1000,
  };

  function handleSubmit(data: { interval: number }) {
    // Update the dataSource object with the new values
    dataSource = {
      ...dataSource,
      interval: data.interval,
    };
  }

  let query200 =
    'sum(http_client_request_duration_seconds_count{"status"=~"2.+|3.+"})';
  let query400 =
    'sum(http_client_request_duration_seconds_count{"status"=~"4.+"})';
  let query500 =
    'sum(http_client_request_duration_seconds_count{"status"=~"5.+"})';
</script>

<main class="bg-gray-50">
  <Header onSubmit={handleSubmit} {dataSource} />
  <div class="grid grid-cols-6 gap-4 max-w-7xl mx-auto px-20 py-5">
    <div class="col-span-2 border shadow-xs rounded-xl p-4 md:p-5 bg-white">
      <TotalCount title="Status 2xx/3xx" query={query200} {dataSource} />
    </div>

    <div class="col-span-2 border shadow-xs rounded-xl p-4 md:p-5 bg-white">
      <TotalCount title="Status 4xx" query={query400} {dataSource} />
    </div>

    <div class="col-span-2 border shadow-xs rounded-xl p-4 md:p-5 bg-white">
      <TotalCount title="Status 5xx" query={query500} {dataSource} />
    </div>

    <div class="col-span-6 border shadow-xs rounded-xl md:p-5 bg-white">
      <EchartsLineChart
        title="Request per second"
        yAxisName="RPS"
        query="sum by(method, path, status) (irate(http_client_request_duration_seconds_count[10s]))"
        {dataSource}
      />
    </div>

    <div class="col-span-6 border shadow-xs rounded-xl md:p-5 bg-white">
      <EchartsLineChart
        title="Request latency(99 percentile)"
        yAxisName="Latency"
        query="histogram_quantile(0.99, sum by (method, path, status, le) (irate(http_client_request_duration_seconds_bucket[10s])))"
        {dataSource}
      />
    </div>
  </div>
</main>

<style>
</style>
