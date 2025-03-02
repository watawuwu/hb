<script lang="ts">
  export let onSubmit: (dataSource: {
    url: string;
    interval: number;
    enableProxy: boolean;
  }) => void;

  export let dataSource: {
    url: string;
    interval: number;
    enableProxy: boolean;
    baseUrl: string;
  };

  let dataSourceUrl = dataSource.url;
  let intervalSec = dataSource.interval / 1000;
  let enableProxy = dataSource.enableProxy;

  let dataSourceUrlReadOnly = false;
  if (dataSource.baseUrl) {
    dataSourceUrlReadOnly = true;
  }

  let dataSourceUrlTitle = "Prometheus URL";
  if (dataSourceUrlReadOnly) {
    dataSourceUrlTitle = "Restricted by server";
  }

  function handleSubmit(event: Event) {
    event.preventDefault();

    onSubmit({
      url: dataSourceUrl,
      interval: intervalSec * 1000,
      enableProxy,
    });
  }
</script>

<header
  class="sticky top-0 inset-x-0 flex flex-wrap md:justify-start md:flex-nowrap z-48 w-full bg-white border-b text-sm py-2.5 lg:ps-[260px]"
>
  <nav class="px-4 sm:px-6 flex basis-full items-center w-full mx-auto">
    <div class="me-5 lg:me-0 font-bold text-2xl">HB</div>

    <div class="flex items-center justify-end ms-auto gap-x-1 md:gap-x-3">
      <form on:submit={handleSubmit} class="max-auto w-full max-w-xl">
        <div class="mx-auto sm:flex sm:space-x-3 bg-white justify-end">
          <div class="flex items-center">
            <input
              type="checkbox"
              id="enable-proxy"
              class="rounded text-pink-500"
              bind:checked={enableProxy}
            />
            <label
              for="enable-proxy"
              class="text-sm text-gray-500 ms-3 dark:text-neutral-400"
              >Proxy</label
            >
          </div>

          <div class="relative w-full max-w-md">
            <label
              for="datasource-url"
              class="absolute bg-white px-1 text-sm block text-gray-800 bottom-7 left-2"
              >Prometheus URL</label
            >
            <div class="mt-1">
              <input
                type="text"
                id="datasource-url"
                placeholder="Enter your Prometheus URL"
                class="border border-gray-300 focus:outline-blue-400 rounded w-full h-10 p-3 pt-5 text-sm w-xs read-only:bg-gray-100 read-only:text-gray-400 read-only:cursor-not-allowed"
                title={dataSourceUrlTitle}
                disabled={dataSourceUrlReadOnly}
                bind:value={dataSourceUrl}
              />
            </div>
          </div>

          <div class="sm:pt-0 sm:ps-3 w-full">
            <div class="relative w-full max-w-md">
              <label
                for="datasource-interval"
                class="absolute bg-white px-1 text-sm block text-gray-800 bottom-7 left-2"
                >Interval(seconds)</label
              >
              <div class="mt-1">
                <input
                  type="number"
                  step="0.1"
                  min="0.0"
                  max="3600"
                  id="datasource-interval"
                  class="border border-gray-300 focus:outline-blue-400 rounded h-10 p-3 size-40"
                  placeholder="0.5"
                  bind:value={intervalSec}
                />
              </div>
            </div>
          </div>

          <div class="whitespace-nowrap pt-2 sm:pt-0 grid sm:block">
            <button
              class="py-3 px-4 inline-flex justify-center items-center gap-x-2 text-sm font-medium rounded-lg border border-transparent bg-blue-600 text-white hover:bg-blue-700 focus:outline-none focus:bg-blue-700 disabled:opacity-50"
            >
              Apply
            </button>
          </div>
        </div>
      </form>
    </div>
  </nav>
</header>
