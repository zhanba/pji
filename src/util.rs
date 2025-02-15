use crate::config::{GitProtocol, GitURI};
use regex::Regex;

pub fn parse_git_url(url: &str) -> Option<GitURI> {
    let ssh_re = Regex::new(r"^git@(?P<host>[^:]+):(?P<user>[^/]+)/(?P<repo>[^/]+)\.git$")
        .expect("Failed to compile SSH regex");
    let http_re = Regex::new(r"^https?://(?P<host>[^/]+)/(?P<user>[^/]+)/(?P<repo>[^/]+)\.git$")
        .expect("Failed to compile HTTP regex");

    if let Some(caps) = ssh_re.captures(url) {
        let hostname = caps.name("host").map(|m| m.as_str()).unwrap_or("");
        let user = caps.name("user").map(|m| m.as_str()).unwrap_or("");
        let repo = caps.name("repo").map(|m| m.as_str()).unwrap_or("");

        Some(GitURI {
            hostname: hostname.to_string(),
            user: user.to_string(),
            repo: repo.to_string(),
            protocol: GitProtocol::SSH,
            uri: url.to_string(),
        })
    } else if let Some(caps) = http_re.captures(url) {
        let hostname = caps.name("host").map(|m| m.as_str()).unwrap_or("");
        let user = caps.name("user").map(|m| m.as_str()).unwrap_or("");
        let repo = caps.name("repo").map(|m| m.as_str()).unwrap_or("");

        Some(GitURI {
            hostname: hostname.to_string(),
            user: user.to_string(),
            repo: repo.to_string(),
            protocol: GitProtocol::HTTP,
            uri: url.to_string(),
        })
    } else {
        None
    }
}
