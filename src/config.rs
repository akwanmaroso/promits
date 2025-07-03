use std::env;

#[derive(Debug)]
pub struct Config {
    pub prometheus_base_url: String,
    pub anthropic_base_url: String,
    pub anthropic_api_key: String,
}

impl Config {
    pub fn from_env() -> Result<Self, Box<dyn std::error::Error>> {
        let prometheus_base_url =
            env::var("PROMETHEUS_BASE_URL").expect("PROMETHEUS_BASE_URL not set");
        let anthropic_base_url =
            env::var("ANTHROPIC_BASE_URL").expect("ANTHROPIC_BASE_URL not set");
        let anthropic_api_key = env::var("ANTHROPIC_API_KEY").expect("ANTHROPIC_API_KEY not set");

        Ok(Config {
            prometheus_base_url,
            anthropic_base_url,
            anthropic_api_key,
        })
    }
}
