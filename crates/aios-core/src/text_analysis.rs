//! 文本和文件路径分析 — 不保留原文内容。
//!
//! 从通知正文提取元数据（长度、文字系统、emoji 检测、语义标签），
//! 从文件路径提取扩展名类别。所有函数均为纯函数，不持有状态。

use aios_spec::{ExtensionCategory, ScriptHint, SemanticHint, TextHint};

// ===== 文本分析 =====

pub(crate) fn analyze_text(text: &str) -> TextHint {
    let length_chars = text.chars().count();
    let is_emoji_only = !text.is_empty() && text.chars().all(is_emoji);

    let script = if text.is_empty() {
        ScriptHint::Unknown
    } else {
        let mut has_latin = false;
        let mut has_hanzi = false;
        let mut has_cyrillic = false;
        let mut has_arabic = false;

        for ch in text.chars() {
            match ch {
                '\u{0041}'..='\u{007A}' | '\u{00C0}'..='\u{024F}' => has_latin = true,
                '\u{4E00}'..='\u{9FFF}'
                | '\u{3400}'..='\u{4DBF}'
                | '\u{3000}'..='\u{303F}'
                | '\u{FF00}'..='\u{FFEF}' => has_hanzi = true,
                '\u{0400}'..='\u{04FF}' | '\u{0500}'..='\u{052F}' => has_cyrillic = true,
                '\u{0600}'..='\u{06FF}'
                | '\u{0750}'..='\u{077F}'
                | '\u{FB50}'..='\u{FDFF}'
                | '\u{FE70}'..='\u{FEFF}' => has_arabic = true,
                _ => {},
            }
        }

        let count = [has_latin, has_hanzi, has_cyrillic, has_arabic]
            .iter()
            .filter(|&&x| x)
            .count();
        match count {
            0 => ScriptHint::Unknown,
            1 if has_latin => ScriptHint::Latin,
            1 if has_hanzi => ScriptHint::Hanzi,
            1 if has_cyrillic => ScriptHint::Cyrillic,
            1 if has_arabic => ScriptHint::Arabic,
            _ => ScriptHint::Mixed,
        }
    };

    TextHint {
        length_chars,
        script,
        is_emoji_only,
    }
}

/// 从通知标题和正文中提取语义标签。
///
/// 关键词匹配在本地完成，不上传原文。
pub(crate) fn extract_semantic_hints(title: &str, text: &str) -> Vec<SemanticHint> {
    let combined = format!("{} {}", title, text).to_lowercase();
    let mut hints = Vec::new();

    // 文件相关
    if contains_any(
        &combined,
        &[
            "文件",
            "file",
            "pdf",
            "doc",
            "docx",
            "xls",
            "xlsx",
            "ppt",
            "pptx",
            "zip",
            "rar",
            "attachment",
            "附件",
        ],
    ) {
        hints.push(SemanticHint::FileMention);
    }
    // 图片相关
    if contains_any(
        &combined,
        &[
            "图片",
            "照片",
            "截图",
            "image",
            "photo",
            "screenshot",
            "jpg",
            "jpeg",
            "png",
            "gif",
            "webp",
            "相册",
        ],
    ) {
        hints.push(SemanticHint::ImageMention);
    }
    // 语音相关
    if contains_any(
        &combined,
        &[
            "语音", "voice", "audio", "mp3", "wav", "aac", "录音", "通话",
        ],
    ) {
        hints.push(SemanticHint::AudioMessage);
    }
    // 链接相关
    if contains_any(&combined, &["http", "https", "www.", "链接", "link", "url"]) {
        hints.push(SemanticHint::LinkAttachment);
    }
    // 被提及 (@我)
    if contains_any(
        &combined,
        &["@你", "@所有人", "提到了你", "mentioned you", "@"],
    ) {
        hints.push(SemanticHint::UserMentioned);
    }
    // 日历/会议
    if contains_any(
        &combined,
        &[
            "会议",
            "meeting",
            "calendar",
            "日历",
            "invitation",
            "邀请",
            "schedule",
            "日程",
        ],
    ) {
        hints.push(SemanticHint::CalendarInvitation);
    }
    // 金融/交易
    if contains_any(
        &combined,
        &[
            "支付",
            "付款",
            "转账",
            "payment",
            "transaction",
            "红包",
            "balance",
            "余额",
        ],
    ) {
        hints.push(SemanticHint::FinancialContext);
    }
    // 验证码
    if contains_any(
        &combined,
        &["验证码", "code", "otp", "验证", "verification", "captcha"],
    ) {
        hints.push(SemanticHint::VerificationCode);
    }

    hints
}

// ===== 文件路径分析 =====

/// 从文件路径中推断扩展名类别。
pub(crate) fn classify_extension(path: &str) -> ExtensionCategory {
    let lower = path.to_lowercase();
    let ext = std::path::Path::new(&lower)
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("");

    match ext {
        "pdf" | "doc" | "docx" | "xls" | "xlsx" | "ppt" | "pptx" | "txt" | "md" | "csv" | "odt"
        | "ods" | "odp" => ExtensionCategory::Document,
        "jpg" | "jpeg" | "png" | "gif" | "webp" | "heic" | "heif" | "bmp" | "svg" | "tiff" => {
            ExtensionCategory::Image
        },
        "mp4" | "mov" | "avi" | "mkv" | "webm" | "flv" | "wmv" | "3gp" => ExtensionCategory::Video,
        "mp3" | "wav" | "aac" | "flac" | "ogg" | "wma" | "m4a" | "opus" => ExtensionCategory::Audio,
        "zip" | "rar" | "7z" | "tar" | "gz" | "bz2" | "xz" | "apk" | "aab" => {
            ExtensionCategory::Archive
        },
        "py" | "js" | "ts" | "rs" | "cpp" | "c" | "h" | "java" | "kt" | "swift" | "go" | "so"
        | "dylib" | "dll" => ExtensionCategory::Code,
        "" => ExtensionCategory::Unknown,
        _ => ExtensionCategory::Other,
    }
}

// ===== 通用工具 =====

fn contains_any(text: &str, keywords: &[&str]) -> bool {
    keywords.iter().any(|kw| text.contains(kw))
}

fn is_emoji(ch: char) -> bool {
    matches!(ch,
        '\u{1F600}'..='\u{1F64F}'   // Emoticons
        | '\u{1F300}'..='\u{1F5FF}' // Misc Symbols and Pictographs
        | '\u{1F680}'..='\u{1F6FF}' // Transport and Map
        | '\u{1F900}'..='\u{1F9FF}' // Supplemental Symbols and Pictographs
        | '\u{2600}'..='\u{26FF}'   // Misc symbols
        | '\u{2700}'..='\u{27BF}'   // Dingbats
        | '\u{FE00}'..='\u{FE0F}'   // Variation Selectors
        | '\u{200D}'                 // ZWJ
        | '\u{1F1E0}'..='\u{1F1FF}' // Flags
    )
}
