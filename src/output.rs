use anyhow::{Context, Result};
use std::collections::BTreeSet;
use std::fs::File;
use std::io::Write;
use std::path::Path;
use whisper_rs::WhisperState;

use crate::voice::profiles::{ass_style_name, hex_to_ass_primary, ProfileStore};

pub struct Segment {
    pub start_cs: i64,
    pub end_cs: i64,
    pub text: String,
    pub speaker: String,
}

pub fn collect_segments(state: &WhisperState) -> Result<Vec<Segment>> {
    let mut segments = Vec::new();
    for seg in state.as_iter() {
        let text = seg.to_str().context("segment text")?.trim().to_string();
        if text.is_empty() {
            continue;
        }
        segments.push(Segment {
            start_cs: seg.start_timestamp(),
            end_cs: seg.end_timestamp(),
            text: normalize_text(&text),
            speaker: String::new(),
        });
    }
    Ok(segments)
}

fn normalize_text(s: &str) -> String {
    s.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// Whisper timestamps are in centiseconds (10 ms units).
fn cs_to_srt_time(cs: i64) -> String {
    let total_ms = cs * 10;
    let hours = total_ms / 3_600_000;
    let minutes = (total_ms % 3_600_000) / 60_000;
    let seconds = (total_ms % 60_000) / 1_000;
    let millis = total_ms % 1_000;
    format!("{hours:02}:{minutes:02}:{seconds:02},{millis:03}")
}

fn cs_to_ass_time(cs: i64) -> String {
    let total_cs = cs.max(0);
    let hours = total_cs / 360_000;
    let minutes = (total_cs % 360_000) / 6_000;
    let seconds = (total_cs % 6_000) / 100;
    let centis = total_cs % 100;
    format!("{hours}:{minutes:02}:{seconds:02}.{centis:02}")
}

fn tagged_text(seg: &Segment) -> String {
    if seg.speaker.is_empty() {
        seg.text.clone()
    } else {
        format!("[{}] {}", seg.speaker, seg.text)
    }
}

pub fn write_srt(path: &Path, segments: &[Segment]) -> Result<()> {
    let mut file = File::create(path).with_context(|| format!("create {}", path.display()))?;
    for (idx, seg) in segments.iter().enumerate() {
        writeln!(file, "{}", idx + 1)?;
        writeln!(
            file,
            "{} --> {}",
            cs_to_srt_time(seg.start_cs),
            cs_to_srt_time(seg.end_cs)
        )?;
        writeln!(file, "{}", tagged_text(seg))?;
        writeln!(file)?;
    }
    Ok(())
}

pub fn write_txt(path: &Path, title: &str, segments: &[Segment]) -> Result<()> {
    let mut file = File::create(path).with_context(|| format!("create {}", path.display()))?;
    writeln!(file, "{title}")?;
    writeln!(file)?;
    for (i, seg) in segments.iter().enumerate() {
        if i > 0 {
            writeln!(file)?;
        }
        write!(file, "{}", tagged_text(seg))?;
    }
    writeln!(file)?;
    Ok(())
}

pub fn write_ass(path: &Path, segments: &[Segment], store: &ProfileStore) -> Result<()> {
    let mut file = File::create(path).with_context(|| format!("create {}", path.display()))?;

    writeln!(file, "[Script Info]")?;
    writeln!(file, "Title: Transcribe")?;
    writeln!(file, "ScriptType: v4.00+")?;
    writeln!(file, "WrapStyle: 0")?;
    writeln!(file, "ScaledBorderAndShadow: yes")?;
    writeln!(file, "YCbCr Matrix: None")?;
    writeln!(file)?;
    writeln!(file, "[V4+ Styles]")?;
    writeln!(
        file,
        "Format: Name, Fontname, Fontsize, PrimaryColour, SecondaryColour, OutlineColour, BackColour, Bold, Italic, Underline, StrikeOut, ScaleX, ScaleY, Spacing, Angle, BorderStyle, Outline, Shadow, Alignment, MarginL, MarginR, MarginV, Encoding"
    )?;

    let mut style_names = BTreeSet::new();
    style_names.insert("Default".to_string());
    for seg in segments {
        if !seg.speaker.is_empty() {
            style_names.insert(ass_style_name(&seg.speaker));
        }
    }
    for p in &store.profiles {
        let name = store.display_name(p);
        style_names.insert(ass_style_name(&name));
    }

    for style in &style_names {
        if style == "Default" {
            writeln!(
                file,
                "Style: Default,Arial,28,&H00FFFFFF,&H000000FF,&H00000000,&H80000000,0,0,0,0,100,100,0,0,1,2,1,2,20,20,20,1"
            )?;
            continue;
        }
        let color = store
            .style_for_speaker(style)
            .or_else(|| {
                store
                    .profiles
                    .iter()
                    .find(|p| ass_style_name(&store.display_name(p)) == *style)
            })
            .map(|p| hex_to_ass_primary(&p.color))
            .unwrap_or_else(|| "&H00FFFFFF".to_string());
        writeln!(
            file,
            "Style: {style},Arial,28,{color},&H000000FF,&H00000000,&H80000000,0,0,0,0,100,100,0,0,1,2,1,2,20,20,20,1"
        )?;
    }

    writeln!(file)?;
    writeln!(file, "[Events]")?;
    writeln!(
        file,
        "Format: Layer, Start, End, Style, Name, MarginL, MarginR, MarginV, Effect, Text"
    )?;

    for seg in segments {
        let style = if seg.speaker.is_empty() {
            style_names
                .iter()
                .next()
                .cloned()
                .unwrap_or_else(|| "Default".to_string())
        } else {
            ass_style_name(&seg.speaker)
        };
        let text = seg.text.replace('\n', "\\N");
        writeln!(
            file,
            "Dialogue: 0,{},{},{},,0,0,0,,{}",
            cs_to_ass_time(seg.start_cs),
            cs_to_ass_time(seg.end_cs),
            style,
            text
        )?;
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::voice::profiles::{hex_to_ass_primary, ProfileStore, VoiceProfile};
    use tempfile::TempDir;

    #[test]
    fn normalize_collapses_whitespace() {
        assert_eq!(normalize_text("  hello   world  "), "hello world");
    }

    #[test]
    fn time_formatters_cover_edges() {
        assert_eq!(cs_to_srt_time(0), "00:00:00,000");
        assert_eq!(cs_to_srt_time(360_000), "01:00:00,000");
        assert_eq!(cs_to_ass_time(-1), "0:00:00.00");
    }

    #[test]
    fn write_ass_with_profile_styles() {
        let tmp = TempDir::new().unwrap();
        let store = ProfileStore {
            default_speaker: "DM".to_string(),
            match_threshold: 0.5,
            profiles: vec![VoiceProfile {
                id: "p1".to_string(),
                name: Some("Alice".to_string()),
                color: "#FF0000".to_string(),
                labeled: true,
                embeddings: vec![],
                clips: vec![],
            }],
        };
        let segments = vec![Segment {
            start_cs: 10,
            end_cs: 50,
            text: "line\nbreak".to_string(),
            speaker: "Alice".to_string(),
        }];
        write_ass(&tmp.path().join("t.ass"), &segments, &store).unwrap();
        let body = std::fs::read_to_string(tmp.path().join("t.ass")).unwrap();
        assert!(body.contains(&hex_to_ass_primary("#FF0000")));
    }
}
