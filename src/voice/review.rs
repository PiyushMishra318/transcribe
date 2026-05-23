use super::profiles::{ass_style_name, ProfileStore};
use anyhow::{Context, Result};
use std::fs::File;
use std::io::Write;
use std::path::Path;

pub fn write_html(voices_dir: &Path, store: &ProfileStore) -> Result<()> {
    let path = voices_dir.join("review.html");
    let mut f = File::create(&path).with_context(|| format!("create {}", path.display()))?;

    writeln!(f, "<!DOCTYPE html>")?;
    writeln!(f, "<html lang=\"en\"><head>")?;
    writeln!(f, "<meta charset=\"utf-8\">")?;
    writeln!(f, "<title>Voice profile review</title>")?;
    writeln!(
        f,
        "<style>body{{font-family:system-ui,sans-serif;margin:2rem;max-width:56rem}}.card{{border:1px solid #ccc;border-radius:8px;padding:1rem;margin:1rem 0}}.swatch{{display:inline-block;width:1rem;height:1rem;border-radius:2px;vertical-align:middle;margin-right:.5rem}}audio{{width:100%;margin:.25rem 0}}</style>"
    )?;
    writeln!(f, "</head><body>")?;
    writeln!(f, "<h1>Voice profiles</h1>")?;
    writeln!(
        f,
        "<p>Label with <code>transcribe voice label &lt;id&gt; \"Name\" --color \"#RRGGBB\"</code></p>"
    )?;

    for p in &store.profiles {
        let name = p.name.as_deref().unwrap_or("(unlabeled)");
        let style = ass_style_name(name);
        writeln!(f, "<div class=\"card\">")?;
        writeln!(
            f,
            "<h2><span class=\"swatch\" style=\"background:{}\"></span>{} <small>({})</small></h2>",
            p.color, style, p.id
        )?;
        writeln!(
            f,
            "<p>labeled={} · embeddings={} · color={}</p>",
            p.labeled,
            p.embeddings.len(),
            p.color
        )?;
        if p.clips.is_empty() {
            writeln!(f, "<p><em>No clips exported yet.</em></p>")?;
        } else {
            for clip in &p.clips {
                writeln!(
                    f,
                    "<audio controls preload=\"none\" src=\"{}\"></audio>",
                    clip
                )?;
            }
        }
        writeln!(f, "</div>")?;
    }

    writeln!(f, "</body></html>")?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::voice::profiles::{ProfileStore, VoiceProfile};
    use tempfile::TempDir;

    #[test]
    fn write_html_with_and_without_clips() {
        let tmp = TempDir::new().unwrap();
        let voices = tmp.path().join("voices");
        std::fs::create_dir_all(&voices).unwrap();
        let store = ProfileStore {
            default_speaker: "DM".into(),
            match_threshold: 0.5,
            profiles: vec![
                VoiceProfile {
                    id: "p0".into(),
                    name: None,
                    color: "#000000".into(),
                    labeled: false,
                    embeddings: vec![],
                    clips: vec![],
                },
                VoiceProfile {
                    id: "p1".into(),
                    name: Some("Named".into()),
                    color: "#FFFFFF".into(),
                    labeled: true,
                    embeddings: vec![vec![0.0; 4]],
                    clips: vec!["clips/p1_01.wav".into()],
                },
            ],
        };
        write_html(&voices, &store).unwrap();
        assert!(voices.join("review.html").is_file());
    }
}
