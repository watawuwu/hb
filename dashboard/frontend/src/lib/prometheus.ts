
export interface DataItem {
    labels: { [key: string]: string };
    sample: [Date, number];
}

export async function query(query: string, origin: string, proxy: boolean): Promise<DataItem[] | undefined> {

  let url = "";
  if (proxy) {
    const originUrl = `${origin}/api/v1/query?query=${query}`;
    const encodedQuery = encodeURIComponent(originUrl);
    url = `proxy?url=${encodedQuery}`;
  } else {
    const encodedQuery = encodeURIComponent(query);
    url = `${origin}/api/v1/query?query=${encodedQuery}`;
  }

  const response = await fetch(url);
  const result = await response.json();
  if (result.status !== "success") {
    throw result.error;
  }

  let results: DataItem[] = [];

  if (result.data.result.length === 0) {
    return undefined;
  }

  for (const item of result.data.result) {
      let labels = Object.fromEntries(Object.entries(item.metric).map(([key, value]) => [key, String(value)]));

      const sample = item.value;
      const ts = new Date(sample[0] * 1000);
      const value = parseFloat(sample[1]);

      results.push({ labels: labels, sample: [ts, value] });
  }

  return results;
}

export function sortAndFormat(obj: { [key: string]: string }): string {
  // Get the keys and sort them
  const sortedKeys = Object.keys(obj).sort();

  // Combine the keys and values with '=' and make an array
  const keyValuePairs = sortedKeys.map((key) => `${key}=${obj[key]}`);

  // Join the array with ',' and make a string
  return keyValuePairs.join(",");
}
