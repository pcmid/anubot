pub const DEFAULT_GROUP_VERIFICATION_TEXT: &str = "欢迎 {user}！加入 {chat} 需要完成人机验证,\
     请点击下方按钮前往与机器人的私聊,\
     在 {timeout} 分钟内完成验证,超时将被移出群组。";

pub const DEFAULT_BUTTON_LABEL: &str = "前往私聊验证";

pub const DEFAULT_DM_VERIFICATION_TEXT: &str = "你好 {user},\
     请点击下方按钮完成在《{chat}》的人机验证。";

pub const DEFAULT_DM_BUTTON_LABEL: &str = "点击验证";

pub const DM_NO_PENDING: &str = "没有待验证的请求,或已过期。";

pub const CMD_ENABLE: &str = "启用本群的人机验证";
pub const CMD_DISABLE: &str = "停用本群的人机验证";
pub const CMD_SET_TIMEOUT: &str = "设置验证超时(秒,60-3600)";
pub const CMD_SET_WELCOME: &str = "自定义欢迎语(清空则恢复默认)";
pub const CMD_SET_BUTTON: &str = "自定义按钮文字(清空则恢复默认)";
pub const CMD_STATUS: &str = "查看本群验证状态";
pub const CMD_SETTINGS: &str = "在私聊中配置 AI 反垃圾检测";
pub const CMD_BAN: &str = "封禁被回复消息的用户并删除该消息";
pub const CMD_TEST_SPAM: &str = "回复消息后测试 AI 反垃圾检测";

pub const SETTINGS_LINK_LABEL: &str = "在私聊中配置";

pub const SETTINGS_NOT_ADMIN: &str = "你已不是该群管理员,无法配置。";

pub const SETTINGS_GROUP_NOT_REGISTERED: &str = "请先在群里 /enable 后再配置 AI 检测。";

pub const SETTINGS_PROMPT_API_BASE: &str = "请回复新的 OpenAI 兼容 API base URL\
     (例如 https://api.openai.com/v1 或 http://localhost:11434/v1)。";

pub const SETTINGS_PROMPT_API_KEY: &str = "请回复新的 API Key。";

pub const SETTINGS_PROMPT_MODEL: &str = "请回复要使用的模型名,例如 gpt-4o-mini、qwen2.5:7b。";

pub const SETTINGS_PROMPT_SPAM_MESSAGE_LIMIT: &str = "请回复要检查的新成员消息数。";

pub const SETTINGS_PROMPT_SPAM_WINDOW_HOURS: &str = "请回复验证通过后的检查时间窗小时数。";

pub const SETTINGS_PROMPT_SPAM_DELETE_SCORE: &str = "请回复删除消息的垃圾分数阈值(0-100)。";

pub const SETTINGS_PROMPT_SPAM_KICK_SCORE: &str = "请回复直接踢出的垃圾分数阈值(0-100)。";

pub const SETTINGS_PROMPT_SPAM_KICK_THRESHOLD: &str = "请回复累计多少条消息达到删除阈值后踢出。";

pub const SETTINGS_INVALID_URL: &str = "看起来不是合法的 URL,请检查后重发。";

pub const SETTINGS_EMPTY_VALUE: &str = "内容不能为空,请重新回复。";

pub const SETTINGS_INVALID_NUMBER: &str = "请输入有效数字,并确认在允许范围内。";

pub const SETTINGS_TEST_PENDING: &str = "正在测试连通,请稍候...";
pub const SETTINGS_TEST_MISSING_CONFIG: &str =
    "请先填齐 Provider / API URL / API Key / Model 4 项后再测试。";
pub const SETTINGS_TEST_OK: &str = "连通正常。";
pub const SETTINGS_TEST_FAILED_PREFIX: &str = "测试失败:";
pub const SETTINGS_UNSET: &str = "(未设置)";

pub const BTN_SET_PROVIDER: &str = "Provider";
pub const BTN_SET_API_BASE: &str = "API URL";
pub const BTN_SET_API_KEY: &str = "API Key";
pub const BTN_SET_MODEL: &str = "Model";
pub const BTN_SET_SPAM_MESSAGE_LIMIT: &str = "检查消息数";
pub const BTN_SET_SPAM_WINDOW_HOURS: &str = "检查时间窗";
pub const BTN_SET_SPAM_DELETE_SCORE: &str = "删除分数";
pub const BTN_SET_SPAM_KICK_SCORE: &str = "踢出分数";
pub const BTN_SET_SPAM_KICK_THRESHOLD: &str = "累计踢出";
pub const BTN_TEST: &str = "测试连通";

pub const SETTINGS_PROMPT_PICK_PROVIDER: &str = "请选择 AI provider:";

pub const SETTINGS_PROVIDER_SELECTED_PREFIX: &str = "已选择 ";

pub const SETTINGS_COMMAND_PROMPT: &str = "请点击下方按钮在私聊中完成 AI 检测配置:";

pub const REPLY_OK: &str = "操作成功。";
pub const REPLY_INVALID_TIMEOUT: &str = "超时时长无效,允许范围 60-3600 秒。";
pub const REPLY_STATUS_ENABLED: &str = "已启用";
pub const REPLY_STATUS_DISABLED: &str = "已停用";
pub const REPLY_STATUS_TEMPLATE: &str = "验证状态:{state}\n超时设置:{timeout_seconds} 秒\n\
     过去 24 小时已验证:{verified_24h}\n过去 24 小时已拒绝:{declined_24h}";
pub const REPLY_NOT_REGISTERED_TEMPLATE: &str =
    "本群尚未启用验证,请先执行 /enable@{bot_username}。";
pub const REPLY_NOT_SUPERGROUP: &str =
    "本群是普通群组,无法对成员做限制,需要先升级为超级群组 (supergroup)。";
pub const REPLY_BAN_NEED_REPLY: &str = "请回复一条用户消息后使用 /ban。";
pub const REPLY_BAN_NO_USER: &str = "无法识别被回复消息的发送者。";
pub const REPLY_TEST_SPAM_NEED_REPLY: &str = "请回复一条消息后使用 /test_spam。";
pub const REPLY_TEST_SPAM_NO_TEXT: &str = "被回复消息没有可检查的文本。";
pub const REPLY_TEST_SPAM_MISSING_CONFIG: &str = "请先配置完整 AI 检查。";
pub const REPLY_TEST_SPAM_FAILED_TEMPLATE: &str = "AI 检查失败:{error}";

pub const FORCE_REPLY_PLACEHOLDER: &str = "在此输入...";

pub const SETTINGS_AI_CONFIG_TEMPLATE: &str = "AI 反垃圾配置(chat_id={chat})\n\n\
     Provider: {provider}\n\
     API URL:  {base}\n\
     API Key:  {key}\n\
     Model:    {model}\n\
     检查消息数: {limit}\n\
     检查时间窗: {window_hours} 小时\n\
     删除分数: {delete_score}\n\
     踢出分数: {kick_score}\n\
     累计踢出: {kick_threshold} 条\n\n\
     点击下方按钮修改对应字段。\n\
     四项全部填写后,新成员验证通过后的前 {limit} 条消息,或在前 {window_hours} 小时内,将自动经 AI 评分；达到 {delete_score} 分删除,达到 {kick_score} 分直接踢出；累计 {kick_threshold} 条达到删除分数后踢出。";

pub const PROVIDER_BUTTONS: &[(&str, &str)] = &[
    ("OpenAI", "openai"),
    ("OpenAI Responses", "openai_resp"),
    ("Anthropic", "anthropic"),
    ("Gemini", "gemini"),
    ("Ollama", "ollama"),
    ("Groq", "groq"),
    ("xAI", "xai"),
    ("DeepSeek", "deepseek"),
    ("Cohere", "cohere"),
];

pub fn settings_tag(chat_id: i64, field_tag: &str) -> String {
    format!("\n\n[set:{chat_id}:{field_tag}]")
}

pub const SPAM_SYSTEM_PROMPT: &str = "你是 Telegram 群组反垃圾消息助手。\
     判断以下用户消息是否是垃圾消息(典型特征:广告引流、推广联系方式、\
     博彩 / 色情链接、加好友诱导、空泛刷屏、机器人代发等)。\
     只输出一个 0 到 100 的整数分数,分数越高越像垃圾消息。不要任何解释。";

pub const WEB_VERIFY_TITLE: &str = "Anubot 验证";
pub const WEB_VERIFY_LINK_INVALID: &str = "该验证链接无效,请返回群组重新申请。";
pub const WEB_VERIFY_SERVICE_ERROR: &str = "服务异常,请稍后再试。";
pub const WEB_VERIFY_EXPIRED: &str = "本次验证已失效,请返回群组重新申请。";
pub const WEB_VERIFY_UNRESTRICT_FAILED: &str = "解除禁言失败,请稍后再试。";
pub const WEB_VERIFY_OK: &str = "验证通过,请返回群组。";

pub fn fill(template: &str, pairs: &[(&str, &str)]) -> String {
    let mut out = String::with_capacity(template.len());
    let mut rest = template;
    while let Some(start) = rest.find('{') {
        out.push_str(&rest[..start]);
        let after = &rest[start + 1..];
        match after.find('}') {
            Some(end) => {
                let key = &after[..end];
                match pairs.iter().find(|(k, _)| *k == key) {
                    Some((_, value)) => out.push_str(value),
                    None => {
                        out.push('{');
                        out.push_str(key);
                        out.push('}');
                    }
                }
                rest = &after[end + 1..];
            }
            None => {
                out.push_str(&rest[start..]);
                return out;
            }
        }
    }
    out.push_str(rest);
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn substitutes_known_keys() {
        assert_eq!(
            fill(
                "hi {name}, joining {chat}",
                &[("name", "Alice"), ("chat", "Gentoo zh")]
            ),
            "hi Alice, joining Gentoo zh"
        );
    }

    #[test]
    fn preserves_unknown_keys() {
        assert_eq!(fill("{a} and {b}", &[("a", "X")]), "X and {b}");
    }

    #[test]
    fn does_not_recurse_into_substituted_values() {
        assert_eq!(fill("{a}", &[("a", "{b}"), ("b", "X")]), "{b}");
    }

    #[test]
    fn preserves_lone_open_brace() {
        assert_eq!(fill("hello {", &[]), "hello {");
    }
}
