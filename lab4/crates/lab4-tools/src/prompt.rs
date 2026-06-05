//! Prompt dataset loading and validation.

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::error::{Lab4Error, Lab4Result};
use crate::jsonl;

/// One prompt case used by Lab4 experiments.
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
pub struct PromptCase {
    /// Stable prompt identifier, for example `os-001`.
    pub id: String,
    /// Prompt category, for example `os`, `summary`, or `code`.
    pub category: String,
    /// Prompt text sent to the model.
    pub prompt: String,
    /// Optional generation limit for this prompt.
    pub max_tokens: Option<u32>,
}

/// Count summary for a prompt dataset.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PromptSummary {
    /// Total number of valid prompts.
    pub total: usize,
    /// Number of prompts grouped by category.
    pub categories: BTreeMap<String, usize>,
}

/// Loads and validates a prompt JSONL file.
///
/// # Errors
///
/// Returns an error if the JSONL file cannot be read, any line cannot be
/// decoded as [`PromptCase`], or the prompt records fail validation.
pub fn load_prompts(path: &Path) -> Lab4Result<Vec<PromptCase>> {
    let prompts = jsonl::read_jsonl(path)?;
    validate_prompts(&prompts)?;
    Ok(prompts)
}

/// Validates prompt identifiers, categories, text, and token limits.
///
/// # Errors
///
/// Returns [`Lab4Error::InvalidPrompt`] for empty fields or invalid token
/// limits, and [`Lab4Error::DuplicatePromptId`] when ids are reused.
pub fn validate_prompts(prompts: &[PromptCase]) -> Lab4Result<()> {
    let mut ids = BTreeSet::new();
    for prompt in prompts {
        validate_prompt(prompt)?;
        if !ids.insert(prompt.id.clone()) {
            return Err(Lab4Error::DuplicatePromptId(prompt.id.clone()));
        }
    }
    Ok(())
}

/// Builds a count summary for a valid prompt slice.
#[must_use]
pub fn summarize_prompts(prompts: &[PromptCase]) -> PromptSummary {
    let mut categories = BTreeMap::new();
    for prompt in prompts {
        let count = categories.entry(prompt.category.clone()).or_insert(0);
        *count += 1;
    }
    PromptSummary {
        total: prompts.len(),
        categories,
    }
}

fn validate_prompt(prompt: &PromptCase) -> Lab4Result<()> {
    if prompt.id.trim().is_empty() {
        return Err(Lab4Error::InvalidPrompt {
            id: "<empty>".to_owned(),
            reason: "id must not be empty".to_owned(),
        });
    }
    if prompt.category.trim().is_empty() {
        return Err(Lab4Error::InvalidPrompt {
            id: prompt.id.clone(),
            reason: "category must not be empty".to_owned(),
        });
    }
    if prompt.prompt.trim().is_empty() {
        return Err(Lab4Error::InvalidPrompt {
            id: prompt.id.clone(),
            reason: "prompt text must not be empty".to_owned(),
        });
    }
    if matches!(prompt.max_tokens, Some(0)) {
        return Err(Lab4Error::InvalidPrompt {
            id: prompt.id.clone(),
            reason: "max_tokens must be greater than zero".to_owned(),
        });
    }
    Ok(())
}

#[cfg(test)]
#[allow(non_snake_case)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_prompts__rejects_duplicate_id() {
        let prompts = vec![
            PromptCase {
                id: "same".to_owned(),
                category: "os".to_owned(),
                prompt: "one".to_owned(),
                max_tokens: Some(32),
            },
            PromptCase {
                id: "same".to_owned(),
                category: "os".to_owned(),
                prompt: "two".to_owned(),
                max_tokens: Some(32),
            },
        ];

        let result = validate_prompts(&prompts);
        assert!(matches!(result, Err(Lab4Error::DuplicatePromptId(id)) if id == "same"));
    }

    #[test]
    fn test_summarize_prompts__counts_categories() {
        let prompts = vec![
            PromptCase {
                id: "a".to_owned(),
                category: "os".to_owned(),
                prompt: "one".to_owned(),
                max_tokens: None,
            },
            PromptCase {
                id: "b".to_owned(),
                category: "code".to_owned(),
                prompt: "two".to_owned(),
                max_tokens: None,
            },
            PromptCase {
                id: "c".to_owned(),
                category: "os".to_owned(),
                prompt: "three".to_owned(),
                max_tokens: None,
            },
        ];

        let summary = summarize_prompts(&prompts);
        assert_eq!(summary.total, 3);
        assert_eq!(summary.categories.get("os"), Some(&2));
        assert_eq!(summary.categories.get("code"), Some(&1));
    }
}
