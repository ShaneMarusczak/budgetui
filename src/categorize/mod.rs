use anyhow::Result;
use regex::Regex;

use crate::models::ImportRule;

pub(crate) struct Categorizer {
    rules: Vec<CompiledRule>,
}

struct CompiledRule {
    pattern: String,
    regex: Option<Regex>,
    category_id: i64,
    is_regex: bool,
}

impl Categorizer {
    pub(crate) fn new(rules: &[ImportRule]) -> Self {
        let compiled = rules
            .iter()
            .map(|r| {
                let regex = if r.is_regex {
                    Regex::new(&r.pattern).ok()
                } else {
                    None
                };
                CompiledRule {
                    pattern: r.pattern.to_lowercase(),
                    regex,
                    category_id: r.category_id,
                    is_regex: r.is_regex,
                }
            })
            .collect();

        Self { rules: compiled }
    }

    pub(crate) fn categorize(&self, description: &str) -> Option<i64> {
        let desc_lower = description.to_lowercase();

        for rule in &self.rules {
            let matched = if rule.is_regex {
                rule.regex
                    .as_ref()
                    .is_some_and(|re| re.is_match(description))
            } else {
                desc_lower.contains(&rule.pattern)
            };

            if matched {
                return Some(rule.category_id);
            }
        }

        None
    }

    pub(crate) fn categorize_batch(&self, transactions: &mut [crate::models::Transaction]) {
        for txn in transactions.iter_mut() {
            if txn.category_id.is_none() {
                txn.category_id = self.categorize(&txn.original_description);
            }
        }
    }
}

/// Suggest a new rule based on a description and category assignment.
pub(crate) fn suggest_rule(description: &str) -> Result<String> {
    // Extract the most likely merchant/vendor name
    let cleaned = description
        .to_uppercase()
        .replace(|c: char| c.is_ascii_digit(), "")
        .replace('#', "")
        .replace('*', " ")
        .trim()
        .to_string();

    // Take the first meaningful word(s)
    let words: Vec<&str> = cleaned.split_whitespace().collect();
    let pattern = if words.len() >= 2 {
        format!("{} {}", words[0], words[1])
    } else if !words.is_empty() {
        words[0].to_string()
    } else {
        description.to_string()
    };

    Ok(pattern.to_lowercase())
}

#[cfg(test)]
mod tests;
