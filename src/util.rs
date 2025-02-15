use regex::Regex;

pub fn parse_git_url(url: &str) -> Option<(String, String, String)> {
    let ssh_re = Regex::new(r"^git@(?P<host>[^:]+):(?P<user>[^/]+)/(?P<repo>[^/]+)\.git$")
        .expect("Failed to compile SSH regex");
    let http_re = Regex::new(r"^https?://(?P<host>[^/]+)/(?P<user>[^/]+)/(?P<repo>[^/]+)\.git$")
        .expect("Failed to compile HTTP regex");

    if let Some(caps) = ssh_re.captures(url) {
        let hostname = caps.name("host").map(|m| m.as_str()).unwrap_or("");
        let user = caps.name("user").map(|m| m.as_str()).unwrap_or("");
        let repo = caps.name("repo").map(|m| m.as_str()).unwrap_or("");

        Some((hostname.to_string(), user.to_string(), repo.to_string()))
    } else if let Some(caps) = http_re.captures(url) {
        let hostname = caps.name("host").map(|m| m.as_str()).unwrap_or("");
        let user = caps.name("user").map(|m| m.as_str()).unwrap_or("");
        let repo = caps.name("repo").map(|m| m.as_str()).unwrap_or("");

        Some((hostname.to_string(), user.to_string(), repo.to_string()))
    } else {
        None
    }
}
