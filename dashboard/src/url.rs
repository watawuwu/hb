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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_url_without_query() -> Result<()> {
        let url = "https://example.com";
        let parsed = parse_url(url)?;
        assert_eq!(parsed.as_str(), "https://example.com/");
        Ok(())
    }

    #[test]
    fn test_parse_url_with_simple_query() -> Result<()> {
        let url = "https://example.com?key=value";
        let parsed = parse_url(url)?;
        assert_eq!(parsed.as_str(), "https://example.com/?key=value");
        Ok(())
    }

    #[test]
    fn test_parse_url_with_special_chars() -> Result<()> {
        let url = "https://example.com?key=hello world&other=123!@#";
        let parsed = parse_url(url)?;
        assert_eq!(
            parsed.as_str(),
            "https://example.com/?key=hello%20world&other=123%21%40%23"
        );
        Ok(())
    }

    #[test]
    fn test_parse_url_with_empty_value() -> Result<()> {
        let url = "https://example.com?key=";
        let parsed = parse_url(url)?;
        assert_eq!(parsed.as_str(), "https://example.com/?key=");
        Ok(())
    }

    #[test]
    fn test_parse_url_with_multiple_params() -> Result<()> {
        let url = "https://example.com?key1=value1&key2=value2";
        let parsed = parse_url(url)?;
        assert_eq!(
            parsed.as_str(),
            "https://example.com/?key1=value1&key2=value2"
        );
        Ok(())
    }

    #[test]
    fn test_parse_url_with_japanese_chars() -> Result<()> {
        let url = "https://example.com?key=こんにちは";
        let parsed = parse_url(url)?;
        assert_eq!(
            parsed.as_str(),
            "https://example.com/?key=%E3%81%93%E3%82%93%E3%81%AB%E3%81%A1%E3%81%AF"
        );
        Ok(())
    }

    #[test]
    fn test_parse_url_with_nested_prometheus_query() -> Result<()> {
        let url = "https://example.com?url=http://localhost:9090/api/v1/query?query=sum(http_client_request_duration_seconds_count{\"status\"=~\"2.+|3.+\"})";
        let parsed = parse_url(url)?;
        assert_eq!(
            parsed.as_str(),
            "https://example.com/?url=http%3A%2F%2Flocalhost%3A9090%2Fapi%2Fv1%2Fquery%3Fquery%3Dsum%28http%5Fclient%5Frequest%5Fduration%5Fseconds%5Fcount%7B%22status%22%3D%7E%222%2E%2B%7C3%2E%2B%22%7D%29"
        );
        Ok(())
    }
}
