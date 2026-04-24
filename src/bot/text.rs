pub const DEFAULT_GROUP_VERIFICATION_TEXT: &str = "欢迎 {user}！加入 {chat} 需要完成人机验证,\
     请点击下方按钮前往与机器人的私聊,\
     在 {timeout} 分钟内完成验证,超时将被移出群组。";

pub const DEFAULT_BUTTON_LABEL: &str = "👉 前往私聊验证";

pub const DEFAULT_DM_VERIFICATION_TEXT: &str = "你好 {user},\
     请点击下方按钮完成在《{chat}》的人机验证。";

pub const DEFAULT_DM_BUTTON_LABEL: &str = "✅ 点击验证";

pub const DM_NO_PENDING: &str = "没有待验证的请求,或已过期。";

pub fn fill(template: &str, pairs: &[(&str, &str)]) -> String {
    let mut out = template.to_string();
    for (k, v) in pairs {
        out = out.replace(&format!("{{{}}}", k), v);
    }
    out
}
