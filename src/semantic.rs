use std::{
    sync::{
        Arc,
        atomic::{AtomicU64, Ordering},
        mpsc::Sender,
    },
    time::Duration,
};

use eframe::egui;
use reqwest::blocking::Client;
use serde_json::{Value, json};

use crate::storage::Note;

pub enum SearchUpdate {
    Match { generation: u64, note_id: i64 },
    Finished { generation: u64 },
    Error { generation: u64, message: String },
}

pub fn validate_endpoint(endpoint: &str) -> Result<(), String> {
    let url = reqwest::Url::parse(endpoint.trim())
        .map_err(|_| "Enter a full HTTP or HTTPS URL.".to_owned())?;
    if !matches!(url.scheme(), "http" | "https") {
        return Err("The endpoint must use HTTP or HTTPS.".to_owned());
    }
    Ok(())
}

pub fn start_search(
    endpoint: String,
    query: String,
    notes: Vec<Note>,
    generation: u64,
    active_generation: Arc<AtomicU64>,
    updates: Sender<SearchUpdate>,
    ctx: egui::Context,
) {
    std::thread::spawn(move || {
        let client = match Client::builder().timeout(Duration::from_secs(5)).build() {
            Ok(client) => client,
            Err(error) => {
                send(
                    &updates,
                    &ctx,
                    SearchUpdate::Error {
                        generation,
                        message: error.to_string(),
                    },
                );
                return;
            }
        };

        for note in notes {
            if active_generation.load(Ordering::Relaxed) != generation {
                return;
            }
            match classify(&client, &endpoint, &query, &note.body) {
                Ok(true) => send(
                    &updates,
                    &ctx,
                    SearchUpdate::Match {
                        generation,
                        note_id: note.id,
                    },
                ),
                Ok(false) => {}
                Err(message) => {
                    send(
                        &updates,
                        &ctx,
                        SearchUpdate::Error {
                            generation,
                            message,
                        },
                    );
                    return;
                }
            }
        }
        send(&updates, &ctx, SearchUpdate::Finished { generation });
    });
}

fn send(updates: &Sender<SearchUpdate>, ctx: &egui::Context, update: SearchUpdate) {
    let _ = updates.send(update);
    ctx.request_repaint();
}

fn classify(client: &Client, endpoint: &str, query: &str, note: &str) -> Result<bool, String> {
    let response = client
        .post(endpoint)
        .json(&request_body(query, note))
        .send()
        .map_err(|error| format!("Laya request failed: {error}"))?
        .error_for_status()
        .map_err(|error| format!("Laya returned an error: {error}"))?;
    let body: Value = response
        .json()
        .map_err(|error| format!("Could not read Laya response: {error}"))?;
    parse_match(&body)
}

fn request_body(query: &str, note: &str) -> Value {
    json!({
        "state": {
            "query": query,
            "note": note,
        },
        "questions": {
            "match": {
                "type": "noul",
                "instructions": "Is this note relevant to the search query, including paraphrases and related concepts?",
                "criteria": {
                    "false": "The note is unrelated to the search query.",
                    "true": "The note discusses or answers the search query."
                }
            }
        }
    })
}

fn parse_match(body: &Value) -> Result<bool, String> {
    let probability = body
        .pointer("/answers/match/noul")
        .and_then(Value::as_f64)
        .ok_or_else(|| "Laya response is missing answers.match.noul.".to_owned())?;
    if !(0.0..=1.0).contains(&probability) {
        return Err("Laya returned an invalid match probability.".to_owned());
    }
    Ok(probability >= 0.5)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{Read, Write};
    use std::net::TcpListener;

    #[test]
    fn laya_request_and_answer_follow_noul_contract() {
        let request = request_body("garden", "Tomatoes need water");
        assert_eq!(request["state"]["query"], "garden");
        assert_eq!(request["state"]["note"], "Tomatoes need water");
        assert_eq!(request["questions"]["match"]["type"], "noul");
        assert!(parse_match(&json!({"answers": {"match": {"noul": 0.8}}})).unwrap());
        assert!(!parse_match(&json!({"answers": {"match": {"noul": 0.2}}})).unwrap());
        assert!(parse_match(&json!({"answers": {}})).is_err());
    }

    #[test]
    fn endpoint_requires_http_url() {
        assert!(validate_endpoint("http://127.0.0.1:8000/v1/systemone").is_ok());
        assert!(validate_endpoint("https://example.com/v1/systemone").is_ok());
        assert!(validate_endpoint("file:///tmp/laya").is_err());
    }

    #[test]
    fn posts_to_configured_laya_endpoint() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        let server = std::thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            let mut request = [0_u8; 4096];
            let received = stream.read(&mut request).unwrap();
            let text = String::from_utf8_lossy(&request[..received]);
            assert!(text.starts_with("POST /v1/systemone HTTP/1.1"));
            let body = r#"{"answers":{"match":{"noul":0.91}}}"#;
            let response = format!(
                "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                body.len()
            );
            stream.write_all(response.as_bytes()).unwrap();
        });

        let client = Client::builder()
            .timeout(Duration::from_secs(2))
            .build()
            .unwrap();
        let result = classify(
            &client,
            &format!("http://{address}/v1/systemone"),
            "garden",
            "Tomatoes need water",
        );
        server.join().unwrap();
        assert!(result.unwrap());
    }
}
