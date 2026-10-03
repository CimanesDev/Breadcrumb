use std::path::Path;

pub fn zone_identifier(path: &Path) -> (Option<String>, Option<String>, Option<String>) {
    let stream = format!("{}:Zone.Identifier", path.display());
    let Ok(text) = std::fs::read_to_string(stream) else { return (None, None, None) };
    let mut host = None;
    let mut referrer = None;
    for line in text.lines() {
        if let Some(v) = line.strip_prefix("HostUrl=") { host = Some(v.trim().to_string()); }
        if let Some(v) = line.strip_prefix("ReferrerUrl=") { referrer = Some(v.trim().to_string()); }
    }
    let domain = host.as_ref().and_then(|v| url::Url::parse(v).ok())
        .and_then(|v| v.host_str().map(str::to_string));
    (host, referrer, domain)
}

#[cfg(test)]
mod tests {
    #[test]
    fn domain_is_not_guessed_from_invalid_url() {
        assert!(url::Url::parse("not a url").is_err());
    }
}
