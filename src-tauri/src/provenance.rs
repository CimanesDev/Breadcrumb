use std::path::Path;
pub mod chromium;

pub fn zone_identifier(path: &Path) -> (Option<String>, Option<String>, Option<String>) {
    let stream = format!("{}:Zone.Identifier", path.display());
    let Ok(text) = std::fs::read_to_string(stream) else {
        return (None, None, None);
    };
    parse_zone_identifier(&text)
}

pub fn zone_label(path: &Path) -> Option<String> {
    let stream = format!("{}:Zone.Identifier", path.display());
    let text = std::fs::read_to_string(stream).ok()?;
    let zone = text.lines().find_map(|line| {
        let (key, value) = line.trim().split_once('=')?;
        key.trim().eq_ignore_ascii_case("ZoneId").then(|| value.trim().to_string())
    })?;
    Some(match zone.as_str() {
        "0" => "Local computer",
        "1" => "Local network",
        "2" => "Trusted site",
        "3" => "Internet",
        "4" => "Restricted site",
        _ => return None,
    }.to_string())
}

pub fn parse_zone_identifier(text: &str) -> (Option<String>, Option<String>, Option<String>) {
    let mut host = None;
    let mut referrer = None;
    let mut in_zone = false;
    for line in text.lines() {
        let line = line.trim();
        if line.starts_with('[') {
            in_zone = line.eq_ignore_ascii_case("[ZoneTransfer]");
            continue;
        }
        if !in_zone {
            continue;
        }
        let Some((key, value)) = line.split_once('=') else {
            continue;
        };
        let value = value.trim();
        if value.is_empty() {
            continue;
        }
        if key.trim().eq_ignore_ascii_case("HostUrl") {
            host = Some(value.to_string());
        }
        if key.trim().eq_ignore_ascii_case("ReferrerUrl") {
            referrer = Some(value.to_string());
        }
    }
    let domain = host
        .as_ref()
        .and_then(|v| url::Url::parse(v).ok())
        .and_then(|v| v.host_str().map(str::to_string));
    (host, referrer, domain)
}

#[cfg(test)]
mod tests {
    use super::parse_zone_identifier;
    #[test]
    fn parses_zone_metadata_and_domain() {
        let (url, referrer, domain) = parse_zone_identifier("[ZoneTransfer]\r\nZoneId=3\r\nHostUrl=https://cdn.example.com/file.zip\r\nReferrerUrl=https://example.com/page\r\n");
        assert_eq!(url.as_deref(), Some("https://cdn.example.com/file.zip"));
        assert_eq!(referrer.as_deref(), Some("https://example.com/page"));
        assert_eq!(domain.as_deref(), Some("cdn.example.com"));
    }
    #[test]
    fn ignores_unrelated_sections() {
        let (url, _, _) = parse_zone_identifier("[Other]\nHostUrl=https://wrong.example\n");
        assert!(url.is_none());
    }
}
