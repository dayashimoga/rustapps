#![forbid(unsafe_code)]
//! Markdown Document Parser for Wiki-Links, Hashtags, and Citations

use regex::Regex;
use std::sync::OnceLock;

static WIKILINK_RE: OnceLock<Regex> = OnceLock::new();
static TAG_RE: OnceLock<Regex> = OnceLock::new();
static CITATION_RE: OnceLock<Regex> = OnceLock::new();

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WikiLink {
    pub target_title: String,
    pub alias: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParsedDocument {
    pub wiki_links: Vec<WikiLink>,
    pub tags: Vec<String>,
    pub citation_keys: Vec<String>,
}

pub struct DocumentParser;

impl DocumentParser {
    fn wikilink_regex() -> &'static Regex {
        WIKILINK_RE.get_or_init(|| Regex::new(r"\[\[([^\]\|]+)(?:\|([^\]]+))?\]\]").unwrap())
    }

    fn tag_regex() -> &'static Regex {
        TAG_RE.get_or_init(|| Regex::new(r"(?:^|\s)#([a-zA-Z0-9_\-]+)").unwrap())
    }

    fn citation_regex() -> &'static Regex {
        CITATION_RE.get_or_init(|| Regex::new(r"\[@([a-zA-Z0-9_\-]+)\]").unwrap())
    }

    /// Extracts all wiki-links `[[Target]]` or `[[Target|Alias]]` from markdown
    pub fn extract_wiki_links(text: &str) -> Vec<WikiLink> {
        let mut links = Vec::new();
        for cap in Self::wikilink_regex().captures_iter(text) {
            let target_title = cap[1].trim().to_string();
            let alias = cap.get(2).map(|m| m.as_str().trim().to_string());
            if !target_title.is_empty() {
                links.push(WikiLink {
                    target_title,
                    alias,
                });
            }
        }
        links
    }

    /// Extracts `#tags` from text
    pub fn extract_tags(text: &str) -> Vec<String> {
        let mut tags = Vec::new();
        for cap in Self::tag_regex().captures_iter(text) {
            let tag = cap[1].trim().to_lowercase();
            if !tag.is_empty() && !tags.contains(&tag) {
                tags.push(tag);
            }
        }
        tags
    }

    /// Extracts bibliographic citation keys `[@key]` from text
    pub fn extract_citations(text: &str) -> Vec<String> {
        let mut citations = Vec::new();
        for cap in Self::citation_regex().captures_iter(text) {
            let key = cap[1].trim().to_string();
            if !key.is_empty() && !citations.contains(&key) {
                citations.push(key);
            }
        }
        citations
    }

    /// Complete document parse extracting all graph references
    pub fn parse(text: &str) -> ParsedDocument {
        ParsedDocument {
            wiki_links: Self::extract_wiki_links(text),
            tags: Self::extract_tags(text),
            citation_keys: Self::extract_citations(text),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_wiki_link_extraction() {
        let md = "In [[Transformer Architecture]], we utilize [[Multi-Head Attention|MHA]] and #nlp [@vaswani2017].";
        let parsed = DocumentParser::parse(md);

        assert_eq!(parsed.wiki_links.len(), 2);
        assert_eq!(
            parsed.wiki_links[0].target_title,
            "Transformer Architecture"
        );
        assert_eq!(parsed.wiki_links[0].alias, None);
        assert_eq!(parsed.wiki_links[1].target_title, "Multi-Head Attention");
        assert_eq!(parsed.wiki_links[1].alias, Some("MHA".to_string()));

        assert_eq!(parsed.tags, vec!["nlp"]);
        assert_eq!(parsed.citation_keys, vec!["vaswani2017"]);
    }
}
