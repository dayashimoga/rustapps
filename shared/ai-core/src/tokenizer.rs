#![forbid(unsafe_code)]
//! Subword BPE / WordPiece Tokenizer
//!
//! Provides deterministic subword segmentation, vocabulary tokenization,
//! and token ID encoding/decoding for offline neural models.

use std::collections::HashMap;

/// Subword vocabulary and tokenization engine
#[derive(Debug, Clone)]
pub struct BpeTokenizer {
    token_to_id: HashMap<String, u32>,
    id_to_token: HashMap<u32, String>,
    unk_id: u32,
}

impl Default for BpeTokenizer {
    fn default() -> Self {
        Self::new_standard()
    }
}

impl BpeTokenizer {
    /// Initialize with standard English and technical multilingual vocabulary
    pub fn new_standard() -> Self {
        let mut token_to_id = HashMap::new();
        let mut id_to_token = HashMap::new();

        let base_vocab = [
            "[PAD]",
            "[UNK]",
            "[CLS]",
            "[SEP]",
            "[MASK]",
            "the",
            "of",
            "and",
            "to",
            "a",
            "in",
            "for",
            "is",
            "on",
            "that",
            "by",
            "this",
            "with",
            "from",
            "as",
            "at",
            "be",
            "an",
            "are",
            "it",
            "or",
            "not",
            "have",
            "has",
            "was",
            "file",
            "log",
            "vault",
            "secret",
            "error",
            "warn",
            "info",
            "debug",
            "trace",
            "app",
            "user",
            "admin",
            "cloud",
            "data",
            "storage",
            "key",
            "token",
            "auth",
            "server",
            "client",
            "connection",
            "database",
            "query",
            "request",
            "response",
            "cpu",
            "memory",
            "ram",
            "disk",
            "network",
            "system",
            "status",
            "ready",
            "fail",
            "failure",
            "panic",
            "crash",
            "exception",
            "timeout",
            "cluster",
            "trans",
            "late",
            "audio",
            "video",
            "frame",
            "color",
            "track",
            "edit",
            "ing",
            "ed",
            "tion",
            "er",
            "es",
            "s",
            "ly",
            "ment",
            "al",
            "ic",
            "able",
        ];

        for (idx, token) in base_vocab.iter().enumerate() {
            let id = idx as u32;
            token_to_id.insert(token.to_string(), id);
            id_to_token.insert(id, token.to_string());
        }

        // Add single characters
        let mut current_id = base_vocab.len() as u32;
        for c in ' '..='~' {
            let s = c.to_string();
            if !token_to_id.contains_key(&s) {
                token_to_id.insert(s.clone(), current_id);
                id_to_token.insert(current_id, s);
                current_id += 1;
            }
        }

        Self {
            token_to_id,
            id_to_token,
            unk_id: 1, // [UNK]
        }
    }

    /// Tokenize text string into subword token IDs
    pub fn encode(&self, text: &str) -> Vec<u32> {
        let mut token_ids = Vec::new();
        for word in text.split_whitespace() {
            let clean = word.to_lowercase();
            // Greedy longest matching subword
            let mut remaining = clean.as_str();
            while !remaining.is_empty() {
                let mut matched = false;
                for len in (1..=remaining.len()).rev() {
                    let sub = &remaining[..len];
                    if let Some(&id) = self.token_to_id.get(sub) {
                        token_ids.push(id);
                        remaining = &remaining[len..];
                        matched = true;
                        break;
                    }
                }
                if !matched {
                    token_ids.push(self.unk_id);
                    // Skip one unicode character
                    if let Some(c) = remaining.chars().next() {
                        remaining = &remaining[c.len_utf8()..];
                    } else {
                        break;
                    }
                }
            }
        }
        token_ids
    }

    /// Decode sequence of token IDs back into string
    pub fn decode(&self, token_ids: &[u32]) -> String {
        let mut result = String::new();
        for &id in token_ids {
            if let Some(token) = self.id_to_token.get(&id) {
                if token.starts_with('[') && token.ends_with(']') {
                    continue;
                }
                if !result.is_empty() && !token.starts_with("##") {
                    result.push(' ');
                }
                result.push_str(token.trim_start_matches("##"));
            }
        }
        result
    }

    /// Vocabulary size
    pub fn vocab_size(&self) -> usize {
        self.token_to_id.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bpe_tokenizer_roundtrip() {
        let tokenizer = BpeTokenizer::new_standard();
        let input = "file storage error connection";
        let tokens = tokenizer.encode(input);
        assert!(!tokens.is_empty());
        let decoded = tokenizer.decode(&tokens);
        assert_eq!(decoded, input);
    }
}
