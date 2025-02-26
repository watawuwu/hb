<script lang="ts">
  import { query, sortAndFormat } from "../prometheus";
  import {
    setAsyncInterval,
    clearAsyncInterval,
    displayTimeRange,
  } from "../time";
  import { onMount } from "svelte";
  import * as echarts from "echarts";

  export let prometheusQuery: string;
  export let title: string;
  export let yAxisName: string;

  const prometheusUrl =
    (window as any).APP_CONFIG?.PROMETHEUS_URL || "http://localhost:9090";

  type EChartsOption = echarts.EChartsOption;

  let chartContainer: HTMLDivElement | null = null;

  interface DataItem {
    name: string;
    value: [Date, number];
  }
  // {
  //   name: string,
  //   type: 'line',
  //   data: [string, number][],
  // }
  let dts: {
    [key: string]: { name: string; data: DataItem[] };
  } = {};

  let defaultSeries: echarts.SeriesOption = {
    type: "line",
    showSymbol: false,
    lineStyle: {
      width: 3,
    },
  };

  var option: EChartsOption;

  option = {
    title: {
      text: title,
      left: "center",
      textStyle: {
        fontSize: 12,
        color: "#666666",
      },
    },
    tooltip: {
      trigger: "axis",
      axisPointer: {
        animation: false,
      },
      valueFormatter: (value) => {
        if (typeof value === "number") {
          return value.toFixed(3);
        }
        return value?.toString() || "";
      },
    },
    xAxis: {
      name: "Timestamp",
      nameLocation: "middle",
      nameGap: 55,
      type: "time",
      splitLine: {
        show: false,
      },
      axisLabel: {
        rotate: 45, // Rotate labels by 45 degrees
      },
    },
    yAxis: {
      name: yAxisName,
      nameLocation: "middle",
      nameGap: 50,
      type: "value",
      splitLine: {
        show: true,
      },
    },
    legend: {
      data: [], // Initial empty
      top: "20px", // Place legend at the top of the chart
      icon: "roundRect", // Round icon
      textStyle: {
        fontSize: 12,
        color: "#666666",
      },
    },
    grid: {
      left: 70, // Set left margin
    },
    series: [],
  };

  let intervalId: number;

  onMount(() => {
    if (chartContainer) {
      const chart = echarts.init(chartContainer);

      chart.setOption(option);

      intervalId = setAsyncInterval(async function () {
        let items = await query(prometheusQuery, prometheusUrl);

        let range = displayTimeRange();

        if (items === undefined || items.length === 0) {
          // Get the Date of the last inserted data in dts

          let options = {};
          if (Object.keys(dts).length > 0) {
            let dt = Object.values(dts)[0];
            const lastDate = dt.data[dt.data.length - 1].value[0];
            if (lastDate.getTime() < range[0]) {
              dts = {};
              options = {
                series: [],
              };
            }
          }

          chart.setOption<EChartsOption>({
            ...options,
            xAxis: {
              min: range[0],
              max: range[1],
            },
            legend: {
              data: Object.keys(dts),
            },
          });
          return;
        }

        items.forEach((item) => {
          let label = sortAndFormat(item.labels);
          let dataitem: DataItem = {
            name: label,
            value: [item.sample[0], item.sample[1]],
          };

          let dt = dts[label];

          if (dt === undefined) {
            dt = {
              name: label,
              data: [dataitem],
            };
            dts[label] = dt;
          }

          dt.data.push(dataitem);
        });

        for (let dataset of Object.values(dts)) {
          if (dataset.data.length > 250) {
            dataset.data.shift();
          }
        }

        let series = Object.values(dts).map((dataset) => {
          let series = {
            ...defaultSeries,
            name: dataset.name,
            data: dataset.data,
          };
          return series;
        });

        chart.setOption<EChartsOption>({
          xAxis: {
            min: range[0],
            max: range[1],
          },
          legend: {
            data: Object.keys(dts),
          },
          series: series as echarts.SeriesOption[],
        });
      }, 300);

      window.addEventListener("resize", () => {
        if (chart) {
          chart.resize();
        }
      });

      return () => {
        clearAsyncInterval(intervalId);
        chart.dispose();
      };
    }
  });
</script>

<div bind:this={chartContainer} style="height: 400px;"></div>
