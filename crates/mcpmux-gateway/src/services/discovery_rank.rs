//! Shared ranking and fuzzy-match helpers for discovery indexes.

use std::collections::{HashMap, HashSet};

use tracing::debug;

/// Boost applied when every query token appears in the document haystack.
const AND_MATCH_BOOST: f64 = 1.0;

/// Optional tracing context for tool search ranking.
pub struct RankTraceContext<'a> {
    pub query_id: &'a str,
}

/// Tokenize text for TF-IDF scoring.
fn tokenize(text: &str) -> Vec<String> {
    // TODO(stopwords): filter common stop tokens (e.g. "a", "on", "the") before
    // overlap matching — intent queries like "post a comment on a jira issue" currently
    // match almost every tool via single-letter tokens.
    text.to_lowercase()
        .split(|c: char| !c.is_alphanumeric())
        .filter(|token| !token.is_empty())
        .map(String::from)
        .collect()
}

/// Return true when at least one query token appears in `haystack`.
fn matches_token_overlap(query_tokens: &[String], haystack: &str) -> bool {
    if query_tokens.is_empty() {
        return true;
    }
    let doc_tokens: HashSet<String> = tokenize(haystack).into_iter().collect();
    query_tokens.iter().any(|token| doc_tokens.contains(token))
}

/// Return true when every query token appears in `haystack`.
fn all_tokens_present(query_tokens: &[String], haystack: &str) -> bool {
    if query_tokens.is_empty() {
        return false;
    }
    let doc_tokens: HashSet<String> = tokenize(haystack).into_iter().collect();
    query_tokens.iter().all(|token| doc_tokens.contains(token))
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

/// Lexical relevance score (TF-IDF plus AND-match boost) for hybrid fusion.
pub fn lexical_score(query: &str, document: &str, corpus: &[String]) -> f64 {
    let query_tokens = tokenize(query);
    let base = tf_idf_score(query, document, corpus);
    if all_tokens_present(&query_tokens, document) {
        base + AND_MATCH_BOOST
    } else {
        base
    }
}

/// Filter haystacks by optional token-overlap query and optional server id, then rank.
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
    filter_and_rank_inner(entries, query, server_id, server_id_fn, haystack_fn, None).0
}

/// Like [`filter_and_rank`] but emits a lexical-pass `[search]` trace event.
pub(crate) fn filter_and_rank_traced<'a, T, FServer, FHaystack>(
    entries: &'a [T],
    query: Option<&str>,
    server_id: Option<&str>,
    server_id_fn: FServer,
    haystack_fn: FHaystack,
    trace: &RankTraceContext<'_>,
) -> (Vec<&'a T>, Option<f64>)
where
    FServer: Fn(&T) -> &str,
    FHaystack: Fn(&T) -> String,
{
    filter_and_rank_inner(
        entries,
        query,
        server_id,
        server_id_fn,
        haystack_fn,
        Some(trace),
    )
}

/// Shared filter-and-rank implementation with optional lexical-pass tracing.
fn filter_and_rank_inner<'a, T, FServer, FHaystack>(
    entries: &'a [T],
    query: Option<&str>,
    server_id: Option<&str>,
    server_id_fn: FServer,
    haystack_fn: FHaystack,
    trace: Option<&RankTraceContext<'_>>,
) -> (Vec<&'a T>, Option<f64>)
where
    FServer: Fn(&T) -> &str,
    FHaystack: Fn(&T) -> String,
{
    let query_tokens = query.map(tokenize).unwrap_or_default();
    let mut and_boost_hits = 0usize;

    let mut matched: Vec<&T> = entries
        .iter()
        .filter(|entry| {
            if let Some(sid) = server_id {
                if server_id_fn(entry) != sid {
                    return false;
                }
            }
            if !query_tokens.is_empty() {
                let haystack = haystack_fn(entry);
                if !matches_token_overlap(&query_tokens, &haystack) {
                    return false;
                }
                if all_tokens_present(&query_tokens, &haystack) {
                    and_boost_hits += 1;
                }
            }
            true
        })
        .collect();

    let top_lexical_score = if query.is_some() {
        let corpus: Vec<String> = matched.iter().map(|entry| haystack_fn(entry)).collect();
        matched.sort_by(|a, b| {
            let score_a = lexical_score(query.unwrap(), &haystack_fn(a), &corpus);
            let score_b = lexical_score(query.unwrap(), &haystack_fn(b), &corpus);
            score_b
                .partial_cmp(&score_a)
                .unwrap_or(std::cmp::Ordering::Equal)
                .then_with(|| haystack_fn(a).cmp(&haystack_fn(b)))
        });
        matched
            .first()
            .map(|entry| lexical_score(query.unwrap(), &haystack_fn(entry), &corpus))
    } else {
        matched.sort_by_key(|a| haystack_fn(a));
        None
    };

    if let Some(trace_ctx) = trace {
        if query.is_some() {
            debug!(
                query_id = trace_ctx.query_id,
                tokens = ?query_tokens,
                candidates_after_filter = matched.len(),
                and_boost_hits,
                "[search] lexical pass"
            );
        }
    }

    (matched, top_lexical_score)
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

    struct TestEntry {
        qualified_name: String,
        haystack: String,
    }

    fn test_haystack(entry: &TestEntry) -> String {
        entry.haystack.clone()
    }

    fn test_server_id(_entry: &TestEntry) -> &str {
        "test"
    }

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

    #[test]
    fn token_overlap_matches_hyphenated_tool_name() {
        let entries = vec![TestEntry {
            qualified_name: "canva_list-folder-items".to_string(),
            haystack: "canva_list-folder-items list-folder-items List folder items".to_string(),
        }];
        let matched = filter_and_rank(
            &entries,
            Some("list folder"),
            None,
            |_| "test",
            test_haystack,
        );
        assert_eq!(matched.len(), 1);
        assert_eq!(matched[0].qualified_name, "canva_list-folder-items");
    }

    #[test]
    fn token_overlap_returns_zero_for_nonsense_query() {
        let entries = vec![TestEntry {
            qualified_name: "canva_list-folder-items".to_string(),
            haystack: "canva_list-folder-items list-folder-items List folder items".to_string(),
        }];
        let matched = filter_and_rank(
            &entries,
            Some("xyznotreal"),
            None,
            |_| "test",
            test_haystack,
        );
        assert!(matched.is_empty());
    }

    #[test]
    fn multi_token_ranking_favors_all_tokens_present() {
        let entries = vec![
            TestEntry {
                qualified_name: "partial_list".to_string(),
                haystack: "partial_list list something".to_string(),
            },
            TestEntry {
                qualified_name: "full_list_folder".to_string(),
                haystack: "full_list_folder list folder items".to_string(),
            },
        ];
        let matched = filter_and_rank(
            &entries,
            Some("list folder"),
            None,
            test_server_id,
            test_haystack,
        );
        assert_eq!(matched.len(), 2);
        assert_eq!(matched[0].qualified_name, "full_list_folder");
    }

    #[test]
    fn and_boost_increases_lexical_score() {
        let corpus = vec![
            "partial list something".to_string(),
            "full list folder items".to_string(),
        ];
        let partial = lexical_score("list folder", "partial list something", &corpus);
        let full = lexical_score("list folder", "full list folder items", &corpus);
        assert!(full > partial);
    }
}
