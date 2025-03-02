export function hourMinutesSeconds(date: Date): string {
  return date.toLocaleTimeString("ja-JP", {
    hour: "2-digit",
    minute: "2-digit",
    second: "2-digit",
    hourCycle: "h23",
  });
}

// https://dev.to/jsmccrumb/asynchronous-setinterval-4j69
const asyncIntervals: boolean[] = [];

const runAsyncInterval = async (cb: () => any, interval: number, intervalIndex: number) => {
  await cb();
  if (asyncIntervals[intervalIndex]) {
    setTimeout(() => runAsyncInterval(cb, interval, intervalIndex), interval);
  }
};

export const setAsyncInterval = (cb: () => any, interval: number) => {
  if (cb && typeof cb === "function") {
    const intervalIndex = asyncIntervals.length;
    asyncIntervals.push(true);
    runAsyncInterval(cb, interval, intervalIndex);
    return intervalIndex;
  } else {
    throw new Error('Callback must be a function');
  }
};

export const clearAsyncInterval = (intervalIndex: number) => {
  if (asyncIntervals[intervalIndex]) {
    asyncIntervals[intervalIndex] = false;
  }
};

export const sleep = (ms: number) => new Promise((res) => setTimeout(res, ms))


export class Timer {
  private startTime: number;

  constructor() {
    this.startTime = performance.now();
  }

  elapsedTime(): string {
    return (performance.now() - this.startTime).toFixed(2);
  }
}

export function displayTimeRange(): [number, number] {
  const now = Date.now();
  const startTime = now - (1 * 60 * 1000);
  return [startTime, now];
};
