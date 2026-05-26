//! Shared ranking and fuzzy-match helpers for discovery indexes.

use std::collections::HashMap;

/// Tokenize text for TF-IDF scoring.
fn tokenize(text: &str) -> Vec<String> {
    text.to_lowercase()
        .split(|c: char| !c.is_alphanumeric())
        .filter(|token| !token.is_empty())
        .map(String::from)
        .collect()
}

/// Compute TF-IDF relevance of `query` against `document` given the corpus.
fn tf_idf_score(query: &str, document: &str, corpus: &[String]) -> f64 {
    let query_tokens = tokenize(query);
    if query_tokens.is_empty() {
        return 0.0;
    }

    let doc_tokens = tokenize(document);
    if doc_tokens.is_empty() {
        return 0.0;
    }

    let doc_len = doc_tokens.len() as f64;
    let mut doc_freq: HashMap<String, usize> = HashMap::new();
    for token in &doc_tokens {
        *doc_freq.entry(token.clone()).or_default() += 1;
    }

    let corpus_size = corpus.len().max(1) as f64;
    let mut idf_cache: HashMap<String, f64> = HashMap::new();
    let mut score = 0.0;

    for token in query_tokens {
        let tf = doc_freq.get(&token).copied().unwrap_or(0) as f64 / doc_len;
        if tf == 0.0 {
            continue;
        }

        let idf = *idf_cache.entry(token.clone()).or_insert_with(|| {
            let docs_with_term = corpus
                .iter()
                .filter(|doc| tokenize(doc).contains(&token))
                .count() as f64;
            ((corpus_size + 1.0) / (docs_with_term + 1.0)).ln() + 1.0
        });

        score += tf * idf;
    }

    score
}

/// Filter haystacks by optional substring query and optional server id, then rank.
pub fn filter_and_rank<'a, T, FServer, FHaystack>(
    entries: &'a [T],
    query: Option<&str>,
    server_id: Option<&str>,
    server_id_fn: FServer,
    haystack_fn: FHaystack,
) -> Vec<&'a T>
where
    FServer: Fn(&T) -> &str,
    FHaystack: Fn(&T) -> String,
{
    let query_lower = query.map(|q| q.to_lowercase());
    let mut matched: Vec<&T> = entries
        .iter()
        .filter(|entry| {
            if let Some(sid) = server_id {
                if server_id_fn(entry) != sid {
                    return false;
                }
            }
            if let Some(ref q) = query_lower {
                if !haystack_fn(entry).to_lowercase().contains(q.as_str()) {
                    return false;
                }
            }
            true
        })
        .collect();

    if query.is_some() {
        let corpus: Vec<String> = matched.iter().map(|entry| haystack_fn(entry)).collect();
        matched.sort_by(|a, b| {
            let score_a = tf_idf_score(query.unwrap(), &haystack_fn(a), &corpus);
            let score_b = tf_idf_score(query.unwrap(), &haystack_fn(b), &corpus);
            score_b
                .partial_cmp(&score_a)
                .unwrap_or(std::cmp::Ordering::Equal)
                .then_with(|| haystack_fn(a).cmp(&haystack_fn(b)))
        });
    } else {
        matched.sort_by_key(|a| haystack_fn(a));
    }

    matched
}

/// Return up to `limit` candidates closest to `query` by Levenshtein distance.
pub fn levenshtein_suggestions(query: &str, candidates: &[String], limit: usize) -> Vec<String> {
    if query.is_empty() || candidates.is_empty() || limit == 0 {
        return Vec::new();
    }

    let query_lower = query.to_lowercase();
    let mut scored: Vec<(String, usize)> = candidates
        .iter()
        .map(|candidate| {
            (
                candidate.clone(),
                strsim::levenshtein(&query_lower, &candidate.to_lowercase()),
            )
        })
        .collect();

    scored.sort_by(|a, b| a.1.cmp(&b.1).then_with(|| a.0.cmp(&b.0)));
    scored
        .into_iter()
        .take(limit)
        .map(|(name, _)| name)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tf_idf_ranks_closer_match_first() {
        let entries = ["github_list_issues", "github_get_me", "jira_list_issues"];
        let corpus: Vec<String> = entries.iter().map(|e| e.to_string()).collect();
        let score_list = tf_idf_score("list issues", "github_list_issues List issues", &corpus);
        let score_get = tf_idf_score("list issues", "github_get_me Get current user", &corpus);
        assert!(score_list > score_get);
    }

    #[test]
    fn levenshtein_suggests_near_match() {
        let candidates = vec![
            "github_list_issues".to_string(),
            "github_get_me".to_string(),
        ];
        let suggestions = levenshtein_suggestions("list_isses", &candidates, 2);
        assert_eq!(
            suggestions.first().map(String::as_str),
            Some("github_list_issues")
        );
    }
}
