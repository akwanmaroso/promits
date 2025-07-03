use chrono::{Duration, Utc};
use dotenv::dotenv;
use indicatif::{ProgressBar, ProgressStyle};
use inquire::Select;
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, env, time};

#[derive(Debug, Deserialize)]
struct PromtheusResponse {
    status: String,
    data: PromtheusData,
}

#[derive(Debug, Deserialize, Serialize)]
struct PromtheusData {
    resultType: String,
    result: Vec<PromtheusResult>,
}

#[derive(Debug, Deserialize, Serialize)]
struct PromtheusResult {
    metric: HashMap<String, String>,
    values: Vec<(f64, String)>,
}

#[derive(Debug, Serialize)]
struct Message {
    model: String,
    max_tokens: i64,
    messages: Vec<MessageContent>,
}

#[derive(Debug, Serialize)]
struct MessageContent {
    role: String,
    content: String,
}

#[derive(Debug, Deserialize)]
struct AnthropicBaseResponse {
    role: String,
    model: String,
    usage: AnthropicUsage,
    content: Vec<AnthropicContent>,
}

#[derive(Debug, Deserialize)]
struct AnthropicContent {
    text: String,
}

#[derive(Debug, Deserialize, Serialize)]
struct AnthropicUsage {
    input_tokens: i64,
    output_tokens: i64,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenv().ok();
    let promotheus_base_url = env::var("PROMOTHEUS_BASE_URL").expect("PROMOTHEUS_BASE_URL not set");
    let anthropic_base_url = env::var("ANTHROPIC_BASE_URL").expect("ANTHROPIC_BASE_URL not set");
    let anthropic_api_key = env::var("ANTHROPIC_API_KEY").expect("ANTHROPIC_API_KEY not set");

    let model_opts = vec![
        "claude-opus-4-20250514",
        "claude-sonnet-4-20250514",
        "claude-3-7-sonnet-20250219",
        "claude-3-5-haiku-20241022",
        "claude-3-5-sonnet-20241022",
        "claude-3-5-sonnet-20240620",
        "claude-3-haiku-20240307",
    ];

    let query = "sum(irate(node_cpu_seconds_total{instance='api-prod',job='node_exporter', mode='system'}[5m])) / scalar(count(count(node_cpu_seconds_total{instance='api-prod',job='node_exporter'}) by (cpu)))";

    let client = reqwest::Client::new();

    let metric = fetch_metric(&promotheus_base_url, &client, query).await?;

    let model = match Select::new("Choose model", model_opts).prompt() {
        Ok(choice) => choice,
        Err(_) => panic!("invalid options"),
    };

    let spinner = ProgressBar::new_spinner();
    spinner.set_message("Analyzing Log");
    spinner.set_style(
        ProgressStyle::default_spinner()
            .template("{spinner} {msg}")
            .unwrap()
            .tick_chars("⠋⠙⠹⠸⠼⠴⠦⠧⠇⠏"),
    );
    spinner.enable_steady_tick(time::Duration::from_millis(100));

    let resp_analyze = send_message(
        &anthropic_base_url,
        &anthropic_api_key,
        &client,
        metric.data,
        model,
    )
    .await?;

    spinner.finish_with_message("Analyze Completed");

    println!("Usage Token Input: {}", resp_analyze.usage.input_tokens);
    println!("Usage Token Ouput: {}", resp_analyze.usage.output_tokens);
    println!("{}", resp_analyze.content[0].text);

    Ok(())
}

fn get_range_date(duration: i64) -> (i64, i64) {
    let now = Utc::now();
    let now_timestamp = now.timestamp();

    let start = now - Duration::days(duration);
    let start_timestamp = start.timestamp();

    (start_timestamp, now_timestamp)
}

async fn fetch_metric(
    promotheus_base_url: &str,
    client: &reqwest::Client,
    query: &str,
) -> Result<PromtheusResponse, Box<dyn std::error::Error>> {
    let (start_date, end_date) = get_range_date(8);

    let mut params: HashMap<&str, String> = HashMap::new();
    params.insert("start", start_date.to_string());
    params.insert("end", end_date.to_string());
    params.insert("query", query.to_string());
    params.insert("step", "5m".to_string());

    let url = promotheus_base_url.to_string() + "/query_range";
    let resp = match client
        .post(url)
        .header("Content-Type", "application/x-www-form-urlencoded")
        .form(&params)
        .send()
        .await
    {
        Ok(resp) => resp.json().await?,
        Err(e) => return Err(e.into()),
    };

    Ok(resp)
}

async fn send_message(
    anthropic_base_url: &str,
    anthropic_api_key: &str,
    client: &reqwest::Client,
    msg: PromtheusData,
    model: &str,
) -> Result<AnthropicBaseResponse, Box<dyn std::error::Error>> {
    let msg_parsed = serde_json::to_string_pretty(&msg)?;
    let prompt = format!("
        This is matric cpu from promotheus metric, please analyze the data check any anomalia, and give summarize.
        {}
    ", msg_parsed);

    let messages = vec![MessageContent {
        role: "user".to_string(),
        content: prompt,
    }];

    let req = Message {
        model: model.to_string(),
        max_tokens: 1024,
        messages,
    };

    let url = anthropic_base_url.to_string() + "/messages";
    let resp = match client
        .post(url)
        .header("anthropic-version", "2023-06-01")
        .header("content-type", "application/json")
        .header("x-api-key", anthropic_api_key)
        .json(&req)
        .send()
        .await
    {
        Ok(resp) => resp.json().await?,
        Err(e) => return Err(e.into()),
    };

    Ok(resp)
}
