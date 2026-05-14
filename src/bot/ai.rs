use std::time::Duration;

use genai::adapter::AdapterKind;
use genai::chat::{ChatMessage, ChatOptions, ChatRequest};
use genai::resolver::{AuthData, Endpoint, ServiceTargetResolver};
use genai::{Client, ModelIden, ServiceTarget};
use thiserror::Error;

use crate::bot::text::spam_system_prompt;

const AI_CALL_TIMEOUT: Duration = Duration::from_secs(8);

pub fn parse_adapter_kind(s: &str) -> Option<AdapterKind> {
    match s {
        "openai" => Some(AdapterKind::OpenAI),
        "openai_resp" => Some(AdapterKind::OpenAIResp),
        "anthropic" => Some(AdapterKind::Anthropic),
        "gemini" => Some(AdapterKind::Gemini),
        "ollama" => Some(AdapterKind::Ollama),
        "groq" => Some(AdapterKind::Groq),
        "xai" => Some(AdapterKind::Xai),
        "deepseek" => Some(AdapterKind::DeepSeek),
        "cohere" => Some(AdapterKind::Cohere),
        _ => None,
    }
}

#[derive(Debug, Error)]
pub enum AiError {
    #[error("bad config: {0}")]
    BadConfig(&'static str),
    #[error("bad response: {0}")]
    BadResponse(&'static str),
    #[error("genai: {0}")]
    Genai(#[from] genai::Error),
    #[error("timeout after {seconds}s")]
    Timeout { seconds: u64 },
}

pub async fn check_spam(
    provider: &str,
    api_base: &str,
    api_key: &str,
    model: &str,
    chat_title: &str,
    reply_context: Option<&str>,
    message: &str,
) -> Result<i64, AiError> {
    let text = check_spam_raw(
        provider,
        api_base,
        api_key,
        model,
        chat_title,
        reply_context,
        message,
    )
    .await?;
    let score = parse_spam_score(&text).ok_or(AiError::BadResponse("missing 0-100 score"))?;
    tracing::debug!(
        provider,
        model,
        message_chars = message.chars().count(),
        reply_chars = reply_context.map(|s| s.chars().count()).unwrap_or(0),
        response_chars = text.chars().count(),
        score,
        "AI spam check response"
    );
    Ok(score)
}

pub async fn check_spam_raw(
    provider: &str,
    api_base: &str,
    api_key: &str,
    model: &str,
    chat_title: &str,
    reply_context: Option<&str>,
    message: &str,
) -> Result<String, AiError> {
    let kind = parse_adapter_kind(provider).ok_or(AiError::BadConfig("unknown provider"))?;

    let api_base = api_base.to_string();
    let api_key = api_key.to_string();
    let resolver = ServiceTargetResolver::from_resolver_fn(
        move |st: ServiceTarget| -> Result<ServiceTarget, genai::resolver::Error> {
            let endpoint = Endpoint::from_owned(api_base.clone());
            let auth = AuthData::from_single(api_key.clone());
            let model = ModelIden::new(kind, st.model.model_name);
            Ok(ServiceTarget {
                endpoint,
                auth,
                model,
            })
        },
    );
    let client = Client::builder()
        .with_service_target_resolver(resolver)
        .build();

    let system = spam_system_prompt(chat_title);
    let mut messages = vec![ChatMessage::system(system)];
    if let Some(ctx) = reply_context.filter(|s| !s.trim().is_empty()) {
        messages.push(ChatMessage::user(format!(
            "【上下文,仅供理解,请不要对这一段评分】被审核用户正在回复以下消息:\n{ctx}"
        )));
    }
    messages.push(ChatMessage::user(format!(
        "【被审核内容,请只对以下这一段评分】\n{message}"
    )));
    let req = ChatRequest::new(messages);
    let opts = ChatOptions::default().with_max_tokens(3);
    let resp = match tokio::time::timeout(
        AI_CALL_TIMEOUT,
        client.exec_chat(model, req, Some(&opts)),
    )
    .await
    {
        Ok(r) => r?,
        Err(_) => {
            return Err(AiError::Timeout {
                seconds: AI_CALL_TIMEOUT.as_secs(),
            });
        }
    };
    Ok(resp.first_text().unwrap_or("").to_string())
}

fn parse_spam_score(text: &str) -> Option<i64> {
    let mut current = String::new();
    for ch in text.chars() {
        if ch.is_ascii_digit() {
            current.push(ch);
            continue;
        }
        if let Some(score) = parse_score_digits(&current) {
            return Some(score);
        }
        current.clear();
    }
    parse_score_digits(&current)
}

fn parse_score_digits(digits: &str) -> Option<i64> {
    let score = digits.parse::<i64>().ok()?;
    (0..=100).contains(&score).then_some(score)
}
