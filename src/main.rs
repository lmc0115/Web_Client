use reqwest::blocking::{Client, Response};
use reqwest::header::CONTENT_TYPE;
use serde_json::Value;
use std::error::Error;
use structopt::StructOpt;
use url::Url;

#[derive(StructOpt, Debug)]
#[structopt(name = "curl")]
struct Cli {
    url: String,

    #[structopt(short = "X", long)]
    method: Option<String>,

    #[structopt(short = "d", long)]
    data: Option<String>,

    #[structopt(long)]
    json: Option<String>,
}

fn main() {
    let args = Cli::from_args();

    // Automatically infer POST when -d or --json are used without -X
    let mut method = args.method.clone().unwrap_or_else(|| "GET".to_string());
    if method.eq_ignore_ascii_case("GET") && (args.json.is_some() || args.data.is_some()) {
        method = "POST".to_string();
    }

    println!("Requesting URL: {}", args.url);
    println!("Method: {}", method);

    // Validate and parse the URL
    let parsed = match Url::parse(&args.url) {
        Ok(u) => u,
        Err(e) => {
            handle_url_error(e);
            return;
        }
    };

    // Reject unsupported protocols early
    let scheme = parsed.scheme();
    if scheme != "http" && scheme != "https" {
        println!("Error: The URL does not have a valid base protocol.");
        return;
    }

    let client = Client::new();

    match method.as_str() {
        "POST" => {
            if let Some(json_data) = args.json {
                handle_json_post(&client, &parsed, &json_data);
            } else if let Some(data) = args.data {
                handle_form_post(&client, &parsed, &data);
            } else {
                println!("Error: POST method requires -d or --json data.");
            }
        }
        _ => {
            if let Err(e) = handle_get(&client, &parsed) {
                println!("{}", e);
            }
        }
    }
}

// ---------------- URL ERROR HANDLING ----------------

fn handle_url_error(err: url::ParseError) {
    let msg = err.to_string();

    if msg.contains("relative URL") {
        println!("Error: The URL does not have a valid base protocol.");
    } else if msg.contains("invalid port number") {
        println!("Error: The URL contains an invalid port number.");
    } else if msg.contains("invalid IPv4 address") {
        println!("Error: The URL contains an invalid IPv4 address.");
    } else if msg.contains("invalid IPv6 address") {
        println!("Error: The URL contains an invalid IPv6 address.");
    } else {
        println!("Error: {}", msg);
    }
}

// ---------------- HTTP HANDLERS ----------------

fn handle_get(client: &Client, url: &Url) -> Result<(), Box<dyn Error>> {
    let res = client.get(url.clone()).send();

    match res {
        Ok(r) => print_response(r),
        Err(_) => println!(
            "Error: Unable to connect to the server. Perhaps the network is offline or the server hostname cannot be resolved."
        ),
    }

    Ok(())
}

fn handle_form_post(client: &Client, url: &Url, data: &str) {
    println!("Data: {}", data);
    let form_data: Vec<(&str, &str)> = data
        .split('&')
        .filter_map(|s| s.split_once('='))
        .collect();

    match client.post(url.clone()).form(&form_data).send() {
        Ok(r) => print_response(r),
        Err(_) => println!("Error: Unable to connect to the server."),
    }
}

fn handle_json_post(client: &Client, url: &Url, json_str: &str) {
    println!("JSON: {}", json_str);

    let parsed: Value = match serde_json::from_str(json_str) {
        Ok(p) => p,
        Err(e) => panic!("Invalid JSON: {:?}", e),
    };

    let res = client
        .post(url.clone())
        .header(CONTENT_TYPE, "application/json")
        .json(&parsed)
        .send();

    match res {
        Ok(r) => print_response(r),
        Err(_) => println!("Error: Unable to connect to the server."),
    }
}

// ---------------- RESPONSE HANDLING ----------------

fn print_response(res: Response) {
    let status = res.status();
    if !status.is_success() {
        println!("Error: Request failed with status code: {}.", status.as_u16());
        return;
    }

    let text = res.text().unwrap_or_else(|_| "No response body.".to_string());

    if let Ok(json) = serde_json::from_str::<Value>(&text) {
        let sorted = sort_json_keys(&json);
        println!("Response body (JSON with sorted keys):\n{}", sorted);
    } else {
        println!("Response body:\n{}", text);
    }
}

// Sort JSON keys alphabetically for nice output
fn sort_json_keys(value: &Value) -> String {
    if let Value::Object(map) = value {
        let mut sorted = serde_json::Map::new();
        let mut keys: Vec<_> = map.keys().collect();
        keys.sort();
        for k in keys {
            sorted.insert(k.clone(), map[k].clone());
        }
        serde_json::to_string_pretty(&Value::Object(sorted)).unwrap()
    } else {
        serde_json::to_string_pretty(value).unwrap()
    }
}
