//! Key terms files steer Whisper via initial_prompt.

use std::path::Path;

/// One non-comment, non-empty line from a terms file.
pub fn parse_terms(content: &str) -> Vec<String> {
    content
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty() && !line.starts_with('#'))
        .map(str::to_string)
        .collect()
}

/// Load `{video_stem}.terms.txt` or an explicit path; returns Whisper initial_prompt text.
pub fn load_initial_prompt(terms_path: &Path) -> Option<String> {
    if !terms_path.is_file() {
        return None;
    }
    let content = std::fs::read_to_string(terms_path).ok()?;
    let terms = parse_terms(&content);
    if terms.is_empty() {
        return None;
    }
    Some(terms.join(", "))
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn parse_terms_skips_comments_and_blanks() {
        let raw = "# game names\nSilksong\n\nHornet\n# boss\n";
        assert_eq!(
            parse_terms(raw),
            vec!["Silksong".to_string(), "Hornet".to_string()]
        );
    }

    #[test]
    fn load_initial_prompt_joins_terms() {
        let tmp = TempDir::new().unwrap();
        let path = tmp.path().join("ep.terms.txt");
        std::fs::write(&path, "Silksong\nHornet\n").unwrap();
        assert_eq!(
            load_initial_prompt(&path).as_deref(),
            Some("Silksong, Hornet")
        );
    }

    #[test]
    fn load_initial_prompt_missing_file() {
        let tmp = TempDir::new().unwrap();
        assert!(load_initial_prompt(&tmp.path().join("nope.terms.txt")).is_none());
    }
}
