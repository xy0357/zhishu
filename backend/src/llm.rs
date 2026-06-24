use std::{
    env, fs,
    path::PathBuf,
    time::{SystemTime, UNIX_EPOCH},
};

use serde::Serialize;
use serde_json::Value;
use tokio::process::Command;

use crate::models::Citation;

pub struct LlmAnswer {
    pub answer_text: String,
    pub model_name: String,
    pub confidence_score: f32,
}

#[derive(Serialize)]
struct ChatMessage {
    role: String,
    content: String,
}

#[derive(Serialize)]
struct ChatRequest {
    model: String,
    temperature: f32,
    messages: Vec<ChatMessage>,
}

pub async fn generate_answer(question_text: &str, citations: &[Citation]) -> Option<LlmAnswer> {
    let api_key = env::var("ZHIPU_API_KEY").ok()?.trim().to_string();
    if api_key.is_empty() {
        return None;
    }

    let model_name = env::var("ZHIPU_MODEL").unwrap_or_else(|_| "glm-4.5-flash".to_string());
    let base_url = env::var("ZHIPU_BASE_URL")
        .unwrap_or_else(|_| "https://open.bigmodel.cn/api/paas/v4/chat/completions".to_string());

    let evidence_text = if citations.is_empty() {
        "No matching evidence was retrieved from the knowledge base. State that the knowledge base has no direct basis and suggest the next step.".to_string()
    } else {
        citations
            .iter()
            .map(|citation| {
                format!(
                    "[Evidence {order}] Document: {title} {version}\nSnippet: {snippet}",
                    order = citation.cite_order,
                    title = citation.document_title,
                    version = citation.version_no,
                    snippet = citation.snippet_text
                )
            })
            .collect::<Vec<_>>()
            .join("\n\n")
    };

    let payload = ChatRequest {
        model: model_name.clone(),
        temperature: 0.2,
        messages: vec![
            ChatMessage {
                role: "system".to_string(),
                content: "You are an enterprise knowledge-base assistant. Answer only from the provided evidence. Be concise, actionable, and avoid inventing policies, approvers, deadlines, or system names. Prefer this structure: conclusion, steps, notes.".to_string(),
            },
            ChatMessage {
                role: "user".to_string(),
                content: format!(
                    "Question: {question}\n\nEvidence:\n{evidence}\n\nAnswer strictly based on the evidence above.",
                    question = question_text,
                    evidence = evidence_text
                ),
            },
        ],
    };

    let payload_json = serde_json::to_string(&payload).ok()?;
    let payload_path = write_payload_file(&payload_json).ok()?;
    let output = invoke_powershell_request(&base_url, &api_key, &payload_path)
        .await
        .ok()?;
    let _ = fs::remove_file(&payload_path);

    if !output.status.success() {
        return None;
    }

    let stdout = String::from_utf8(output.stdout).ok()?;
    let body: Value = serde_json::from_str(stdout.trim()).ok()?;
    let content = extract_content(&body)?;
    let normalized = content.trim().replace("\r\n", "\n");
    if normalized.is_empty() {
        return None;
    }

    let confidence_score = if citations.is_empty() {
        0.56
    } else {
        (0.76 + citations.len() as f32 * 0.05).min(0.93)
    };

    Some(LlmAnswer {
        answer_text: normalized,
        model_name,
        confidence_score,
    })
}

fn write_payload_file(payload_json: &str) -> std::io::Result<PathBuf> {
    let millis = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis();
    let path = env::temp_dir().join(format!("zhishu-zhipu-{}-{}.json", std::process::id(), millis));
    fs::write(&path, payload_json)?;
    Ok(path)
}

async fn invoke_powershell_request(
    base_url: &str,
    api_key: &str,
    payload_path: &PathBuf,
) -> std::io::Result<std::process::Output> {
    let escaped_url = base_url.replace('"', "`\"");
    let escaped_key = api_key.replace('"', "`\"");
    let escaped_path = payload_path.display().to_string().replace('"', "`\"");
    let script = format!(
        "$headers = @{{ Authorization = 'Bearer {key}'; 'Content-Type' = 'application/json' }}; \
         $body = Get-Content -Raw -Path \"{path}\"; \
         $response = Invoke-RestMethod -Method Post -Uri \"{url}\" -Headers $headers -Body $body; \
         $response | ConvertTo-Json -Depth 20 -Compress",
        key = escaped_key,
        path = escaped_path,
        url = escaped_url,
    );

    Command::new("powershell")
        .args(["-NoProfile", "-Command", &script])
        .output()
        .await
}

fn extract_content(body: &Value) -> Option<String> {
    let content = &body.get("choices")?.get(0)?.get("message")?.get("content")?;
    if let Some(text) = content.as_str() {
        return Some(text.to_string());
    }

    let items = content.as_array()?;
    let merged = items
        .iter()
        .filter_map(|item| item.get("text").and_then(Value::as_str))
        .collect::<Vec<_>>()
        .join("\n");
    if merged.trim().is_empty() {
        None
    } else {
        Some(merged)
    }
}
