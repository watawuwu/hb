use anyhow::Result;
use percent_encoding::{NON_ALPHANUMERIC, utf8_percent_encode};
use url::Url;

pub fn parse_url(url_str: &str) -> Result<Url> {
    // Split by '?' to get the base URL part and the query string
    let splitted: Vec<&str> = url_str.splitn(2, '?').collect();

    if splitted.len() < 2 {
        return Ok(Url::parse(url_str)?);
    }

    let base_url = splitted[0];
    let query_part = splitted[1]; // => "key1=...&key2=..."

    let encoded_pairs = query_part.split('&').fold(Vec::new(), |mut acc, pair| {
        // Split by '=' to get the key and value
        // If there is no '=', consider the value as empty
        let mut kv = pair.splitn(2, '=');
        let key = kv.next().unwrap_or("");
        let val = kv.next().unwrap_or("");

        // Encode the value to URL-encoded (use the key as is)
        let encoded_val = utf8_percent_encode(val, NON_ALPHANUMERIC);

        // Reconstruct the "key=encoded_val" form
        acc.push(format!("{}={}", key, encoded_val));
        acc
    });

    // Reconstruct the query
    let new_query = encoded_pairs.join("&");

    // Reconstruct the base URL + "?" + encoded query
    let new_url = format!("{}?{}", base_url, new_query);

    Ok(Url::parse(&new_url)?)
}
