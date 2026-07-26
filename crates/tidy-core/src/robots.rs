use texting_robots::Robot;
use url::Url;

use crate::error::{Result, TidyError};
use crate::http::USER_AGENT;

#[derive(Debug, Clone)]
pub struct RobotsRules {
    robot: RobotData,
}

#[derive(Debug, Clone)]
struct RobotData {
    allowed_checker: AllowedChecker,
    pub sitemaps: Vec<String>,
    pub crawl_delay: Option<f32>,
}

#[derive(Debug, Clone)]
struct AllowedChecker {
    // Store the raw robots bytes so we can rebuild Robot (Robot itself isn't Clone).
    // We keep parsed sitemaps/delay separately and recreate Robot on allowed checks
    // via a simpler approach: store the Robot's decisions by wrapping in Arc? 
    // Actually Robot is not Clone. Use Arc<Robot>.
    inner: std::sync::Arc<Robot>,
}

impl RobotsRules {
    pub fn sitemaps(&self) -> &[String] {
        &self.robot.sitemaps
    }

    pub fn crawl_delay(&self) -> Option<f32> {
        self.robot.crawl_delay
    }

    pub fn allowed(&self, url: &Url) -> bool {
        self.robot.allowed_checker.inner.allowed(url.as_str())
    }
}

pub fn parse_robots(bytes: &[u8]) -> Result<RobotsRules> {
    let agent = USER_AGENT
        .split('/')
        .next()
        .unwrap_or("Tidy");
    let robot = Robot::new(agent, bytes)
        .map_err(|error| TidyError::Message(format!("robots.txt parse error: {error}")))?;

    let sitemaps = robot.sitemaps.clone();
    let crawl_delay = robot.delay;

    Ok(RobotsRules {
        robot: RobotData {
            allowed_checker: AllowedChecker {
                inner: std::sync::Arc::new(robot),
            },
            sitemaps,
            crawl_delay,
        },
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_sitemaps_and_disallows() {
        let txt = br#"
User-agent: *
Disallow: /private
Sitemap: https://example.com/sitemap.xml
"#;
        let rules = parse_robots(txt).unwrap();
        assert_eq!(
            rules.sitemaps(),
            &["https://example.com/sitemap.xml".to_string()]
        );
        assert!(!rules.allowed(&Url::parse("https://example.com/private/x").unwrap()));
        assert!(rules.allowed(&Url::parse("https://example.com/blog/x").unwrap()));
    }
}
