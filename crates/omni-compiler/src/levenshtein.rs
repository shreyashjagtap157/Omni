use std::collections::HashMap;
use std::iter::Iterator;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct EditOperation {
    pub op: EditType,
    pub pos: usize,
    pub from: Option<char>,
    pub to: Option<char>,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum EditType {
    Insert,
    Delete,
    Replace,
    Transpose,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EditDistance {
    operations: Vec<EditOperation>,
    distance: usize,
}

impl EditDistance {
    pub fn distance(&self) -> usize {
        self.distance
    }

    pub fn operations(&self) -> &[EditOperation] {
        &self.operations
    }
}

pub fn levenshtein_distance(s1: &str, s2: &str) -> usize {
    let s1_chars: Vec<char> = s1.chars().collect();
    let s2_chars: Vec<char> = s2.chars().collect();
    let len1 = s1_chars.len();
    let len2 = s2_chars.len();

    if len1 == 0 {
        return len2;
    }
    if len2 == 0 {
        return len1;
    }

    let mut matrix = vec![vec![0usize; len2 + 1]; len1 + 1];

    for i in 0..=len1 {
        matrix[i][0] = i;
    }
    for j in 0..=len2 {
        matrix[0][j] = j;
    }

    for i in 1..=len1 {
        for j in 1..=len2 {
            let cost = if s1_chars[i - 1] == s2_chars[j - 1] {
                0
            } else {
                1
            };
            matrix[i][j] = *[
                matrix[i - 1][j] + 1,
                matrix[i][j - 1] + 1,
                matrix[i - 1][j - 1] + cost,
            ]
            .iter()
            .min()
            .unwrap();
        }
    }

    matrix[len1][len2]
}

pub fn damerau_levenshtein_distance(s1: &str, s2: &str) -> usize {
    let s1_chars: Vec<char> = s1.chars().collect();
    let s2_chars: Vec<char> = s2.chars().collect();
    let len1 = s1_chars.len();
    let len2 = s2_chars.len();

    if len1 == 0 {
        return len2;
    }
    if len2 == 0 {
        return len1;
    }

    let mut matrix = vec![vec![0usize; len2 + 1]; len1 + 1];

    for i in 0..=len1 {
        matrix[i][0] = i;
    }
    for j in 0..=len2 {
        matrix[0][j] = j;
    }

    for i in 1..=len1 {
        for j in 1..=len2 {
            let cost = if s1_chars[i - 1] == s2_chars[j - 1] {
                0
            } else {
                1
            };
            matrix[i][j] = *[
                matrix[i - 1][j] + 1,
                matrix[i][j - 1] + 1,
                matrix[i - 1][j - 1] + cost,
            ]
            .iter()
            .min()
            .unwrap();

            if i > 1
                && j > 1
                && s1_chars[i - 1] == s2_chars[j - 2]
                && s1_chars[i - 2] == s2_chars[j - 1]
            {
                matrix[i][j] = matrix[i][j].min(matrix[i - 2][j - 2] + cost);
            }
        }
    }

    matrix[len1][len2]
}

pub fn get_edit_operations(s1: &str, s2: &str) -> EditDistance {
    let s1_chars: Vec<char> = s1.chars().collect();
    let s2_chars: Vec<char> = s2.chars().collect();
    let len1 = s1_chars.len();
    let len2 = s2_chars.len();

    let mut operations = Vec::new();
    let distance = if len1 == 0 {
        for (j, c) in s2_chars.iter().enumerate() {
            operations.push(EditOperation {
                op: EditType::Insert,
                pos: j,
                from: None,
                to: Some(*c),
            });
        }
        len2
    } else if len2 == 0 {
        for (i, c) in s1_chars.iter().enumerate() {
            operations.push(EditOperation {
                op: EditType::Delete,
                pos: i,
                from: Some(*c),
                to: None,
            });
        }
        len1
    } else {
        let mut matrix = vec![vec![0usize; len2 + 1]; len1 + 1];

        for i in 0..=len1 {
            matrix[i][0] = i;
        }
        for j in 0..=len2 {
            matrix[0][j] = j;
        }

        for i in 1..=len1 {
            for j in 1..=len2 {
                let cost = if s1_chars[i - 1] == s2_chars[j - 1] {
                    0
                } else {
                    1
                };
                matrix[i][j] = *[
                    matrix[i - 1][j] + 1,
                    matrix[i][j - 1] + 1,
                    matrix[i - 1][j - 1] + cost,
                ]
                .iter()
                .min()
                .unwrap();
            }
        }

        let mut i = len1;
        let mut j = len2;
        while i > 0 || j > 0 {
            if i > 0 && j > 0 && s1_chars[i - 1] == s2_chars[j - 1] {
                i -= 1;
                j -= 1;
            } else if i > 0 && j > 0 && matrix[i][j] == matrix[i - 1][j - 1] + 1 {
                operations.push(EditOperation {
                    op: EditType::Replace,
                    pos: i - 1,
                    from: Some(s1_chars[i - 1]),
                    to: Some(s2_chars[j - 1]),
                });
                i -= 1;
                j -= 1;
            } else if j > 0 && (i == 0 || matrix[i][j] == matrix[i][j - 1] + 1) {
                operations.push(EditOperation {
                    op: EditType::Insert,
                    pos: j - 1,
                    from: None,
                    to: Some(s2_chars[j - 1]),
                });
                j -= 1;
            } else if i > 0 && (j == 0 || matrix[i][j] == matrix[i - 1][j] + 1) {
                operations.push(EditOperation {
                    op: EditType::Delete,
                    pos: i - 1,
                    from: Some(s1_chars[i - 1]),
                    to: None,
                });
                i -= 1;
            }
        }

        operations.reverse();
        matrix[len1][len2]
    };

    EditDistance {
        operations,
        distance,
    }
}

#[derive(Debug, Clone)]
pub struct DidYouMean {
    candidates: Vec<Candidate>,
    input: String,
}

#[derive(Debug, Clone)]
pub struct Candidate {
    pub name: String,
    pub distance: usize,
    pub suggestion: String,
}

impl DidYouMean {
    pub fn new(input: &str) -> Self {
        DidYouMean {
            candidates: Vec::new(),
            input: input.to_string(),
        }
    }

    pub fn with_candidates<T: IntoIterator<Item = String>>(
        mut self,
        candidates: T,
        max_distance: usize,
    ) -> Self {
        for candidate in candidates {
            let dist = levenshtein_distance(&self.input.to_lowercase(), &candidate.to_lowercase());
            if dist <= max_distance {
                self.candidates.push(Candidate {
                    name: candidate.clone(),
                    distance: dist,
                    suggestion: format!("`{}`", candidate),
                });
            }
        }
        self.candidates.sort_by_key(|c| c.distance);
        self
    }

    pub fn with_candidates_by_prefix<T: IntoIterator<Item = String>>(
        mut self,
        candidates: T,
        prefix: &str,
        max_distance: usize,
    ) -> Self {
        for candidate in candidates {
            if candidate.starts_with(prefix) || prefix.is_empty() {
                let dist = damerau_levenshtein_distance(
                    &self.input.to_lowercase(),
                    &candidate.to_lowercase(),
                );
                if dist <= max_distance {
                    self.candidates.push(Candidate {
                        name: candidate.clone(),
                        distance: dist,
                        suggestion: format!("`{}`", candidate),
                    });
                }
            }
        }
        self.candidates.sort_by_key(|c| c.distance);
        self
    }

    pub fn best(&self) -> Option<&Candidate> {
        self.candidates.first()
    }

    pub fn all(&self) -> &[Candidate] {
        &self.candidates
    }

    pub fn is_empty(&self) -> bool {
        self.candidates.is_empty()
    }

    pub fn len(&self) -> usize {
        self.candidates.len()
    }

    pub fn message(&self) -> Option<String> {
        if self.candidates.is_empty() {
            return None;
        }

        if self.candidates.len() == 1 {
            Some(format!("did you mean {}?", self.candidates[0].suggestion))
        } else if self.candidates.len() <= 3 {
            let names: Vec<String> = self
                .candidates
                .iter()
                .map(|c| c.suggestion.clone())
                .collect();
            Some(format!("did you mean one of {}?", names.join(", ")))
        } else {
            let names: Vec<String> = self
                .candidates
                .iter()
                .take(3)
                .map(|c| c.suggestion.clone())
                .collect();
            Some(format!(
                "did you mean one of {}... ({} others)?",
                names.join(", "),
                self.candidates.len() - 3
            ))
        }
    }

    pub fn help_message(&self) -> Option<String> {
        self.message().map(|msg| {
            if self.candidates.len() == 1 {
                format!("{}\n{}", msg, self.format_help())
            } else {
                msg
            }
        })
    }

    fn format_help(&self) -> String {
        if let Some(candidate) = self.candidates.first() {
            let dist = candidate.distance;
            let operations = get_edit_operations(&self.input, &candidate.name);
            let mut help = String::from("note: closest match requires ");
            help.push_str(&format!(
                "{} edit{}: ",
                dist,
                if dist == 1 { "" } else { "s" }
            ));

            let op_names: Vec<&str> = operations
                .operations
                .iter()
                .map(|op| match op.op {
                    EditType::Insert => "insertion",
                    EditType::Delete => "deletion",
                    EditType::Replace => "replacement",
                    EditType::Transpose => "transpose",
                })
                .collect();
            help.push_str(&op_names.join(", "));
            help
        } else {
            String::new()
        }
    }
}

pub fn levenshtein_suggestions<'a>(
    input: &'a str,
    candidates: &'a [&'a str],
    max_distance: usize,
) -> Vec<(&'a str, usize)> {
    let mut suggestions: Vec<(&'a str, usize)> = candidates
        .iter()
        .filter_map(|c| {
            let dist = levenshtein_distance(&input.to_lowercase(), &c.to_lowercase());
            if dist <= max_distance {
                Some((*c, dist))
            } else {
                None
            }
        })
        .collect();

    suggestions.sort_by_key(|(_, dist)| *dist);
    suggestions
}

pub fn damerau_suggestions<'a>(
    input: &'a str,
    candidates: &'a [&'a str],
    max_distance: usize,
) -> Vec<(&'a str, usize)> {
    let mut suggestions: Vec<(&'a str, usize)> = candidates
        .iter()
        .filter_map(|c| {
            let dist = damerau_levenshtein_distance(&input.to_lowercase(), &c.to_lowercase());
            if dist <= max_distance {
                Some((*c, dist))
            } else {
                None
            }
        })
        .collect();

    suggestions.sort_by_key(|(_, dist)| *dist);
    suggestions
}

pub fn find_similar_identifier(
    input: &str,
    known_ids: &HashMap<String, usize>,
    max_distance: usize,
) -> Option<(String, usize)> {
    let input_lower = input.to_lowercase();
    let mut best: Option<(String, usize)> = None;

    for (id, _def_id) in known_ids {
        let dist = levenshtein_distance(&input_lower, &id.to_lowercase());
        if dist <= max_distance {
            match best {
                None => best = Some((id.clone(), dist)),
                Some((_, best_dist)) if dist < best_dist => best = Some((id.clone(), dist)),
                _ => {}
            }
        }
    }

    best
}

pub fn keyword_suggestion(input: &str, keywords: &[&str]) -> Option<String> {
    let mut best: Option<(String, usize)> = None;
    let input_lower = input.to_lowercase();

    for kw in keywords {
        let dist = levenshtein_distance(&input_lower, &kw.to_lowercase());
        if dist <= 2 {
            match best {
                None => best = Some(((*kw).to_string(), dist)),
                Some((_, best_dist)) if dist < best_dist => best = Some(((*kw).to_string(), dist)),
                _ => {}
            }
        }
    }

    best.map(|(s, _)| s)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_levenshtein_basic() {
        assert_eq!(levenshtein_distance("kitten", "kitten"), 0);
        assert_eq!(levenshtein_distance("kitten", "kittEn"), 1);
        assert_eq!(levenshtein_distance("kitten", "sitting"), 3);
        assert_eq!(levenshtein_distance("kitten", "sit"), 4);
        assert_eq!(levenshtein_distance("", "test"), 4);
        assert_eq!(levenshtein_distance("test", ""), 4);
    }

    #[test]
    fn test_damerau_levenshtein() {
        assert_eq!(damerau_levenshtein_distance("ca", "abc"), 3);
        assert_eq!(damerau_levenshtein_distance("ca", "acb"), 2);
        assert_eq!(damerau_levenshtein_distance("ab", "ba"), 1);
    }

    #[test]
    fn test_did_you_mean() {
        let candidates = vec![
            "fn".to_string(),
            "let".to_string(),
            "mut".to_string(),
            "pub".to_string(),
            "struct".to_string(),
            "enum".to_string(),
            "trait".to_string(),
            "impl".to_string(),
        ];
        let dym = DidYouMean::new("fnn").with_candidates(candidates.clone(), 2);

        assert!(!dym.is_empty());
        assert_eq!(dym.len(), 1);
        assert_eq!(dym.best().unwrap().name, "fn");

        let candidates = vec![
            "fn".to_string(),
            "let".to_string(),
            "mut".to_string(),
            "pub".to_string(),
            "struct".to_string(),
            "enum".to_string(),
            "trait".to_string(),
            "impl".to_string(),
        ];
        let dym = DidYouMean::new("struc").with_candidates(candidates, 2);
        assert_eq!(dym.best().unwrap().name, "struct");
    }

    #[test]
    fn test_did_you_mean_multiple() {
        let candidates = vec!["fn".to_string(), "for".to_string(), "func".to_string()];
        let dym = DidYouMean::new("fnn").with_candidates(candidates, 2);

        assert!(dym.len() >= 2);
        let msg = dym.message().unwrap();
        assert!(msg.contains("fn") || msg.contains(","));
    }

    #[test]
    fn test_keyword_suggestion() {
        let keywords = &["fn", "let", "mut", "pub", "struct", "enum"];

        assert_eq!(keyword_suggestion("fnn", keywords), Some("fn".to_string()));
        assert_eq!(
            keyword_suggestion("mutt", keywords),
            Some("mut".to_string())
        );
        assert_eq!(
            keyword_suggestion("publi", keywords),
            Some("pub".to_string())
        );
        assert_eq!(keyword_suggestion("xyz", keywords), None);
    }

    #[test]
    fn test_suggestions_sorted() {
        let candidates = &["pub", "put", "fn", "fun"];
        let suggestions = levenshtein_suggestions("fnn", candidates, 3);

        assert!(!suggestions.is_empty());
        assert!(suggestions[0].1 <= suggestions.get(1).map(|(_, d)| *d).unwrap_or(0));
    }
}
