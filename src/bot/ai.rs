use genai::adapter::AdapterKind;
use genai::chat::{ChatMessage, ChatOptions, ChatRequest};
use genai::resolver::{AuthData, Endpoint, ServiceTargetResolver};
use genai::{Client, ModelIden, ServiceTarget};
use thiserror::Error;

use crate::bot::text::SPAM_SYSTEM_PROMPT;

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
    #[error("genai: {0}")]
    Genai(#[from] genai::Error),
}

pub async fn check_spam(
    provider: &str,
    api_base: &str,
    api_key: &str,
    model: &str,
    message: &str,
) -> Result<bool, AiError> {
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

    let req = ChatRequest::new(vec![
        ChatMessage::system(SPAM_SYSTEM_PROMPT),
        ChatMessage::user(message),
    ]);
    let opts = ChatOptions::default().with_max_tokens(5);

    tracing::debug!(provider, model, message, "AI spam check request");
    let resp = client.exec_chat(model, req, Some(&opts)).await?;
    let text = resp.first_text().unwrap_or("");
    let is_spam = text.trim().to_lowercase().starts_with("yes");
    tracing::debug!(
        provider,
        model,
        response = text,
        is_spam,
        "AI spam check response"
    );
    Ok(is_spam)
}
