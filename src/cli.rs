//! CLI entrypoint and transcription orchestration.

use crate::audio;
use crate::db::{
    mark_episode_error, mark_episode_profile_built, mark_episode_transcribed, open_db,
    require_project, sync_episodes, sync_profiles,
};
use crate::help;
use crate::label;
use crate::output::{
    apply_word_speakers, collect_segments, collect_words, write_ass, write_srt, write_txt,
    write_words_json, Segment, TimedWord,
};
use crate::terms::load_initial_prompt;
use crate::paths::discover_videos;
use crate::project::resolve_models_dir;
use crate::uninstall;
use crate::voice;
use crate::voice::VoiceEngine;
use anyhow::{bail, Context, Result};
use clap::{Parser, Subcommand};
use serde::Deserialize;
use std::ffi::OsString;
use std::path::{Path, PathBuf};
use crate::gpu::{log_whisper_backend, whisper_context_params};
use whisper_rs::{FullParams, SamplingStrategy, WhisperContext};

#[derive(Parser)]
#[command(
    name = "transcribe",
    version,
    about = "Transcribe episode MP4s to subtitles (optional speaker tagging)",
    long_about = "Operates on the current working directory by default — cd into your episode folder, then run transcribe.",
    disable_help_subcommand = true
)]
#[command(args_conflicts_with_subcommands = true)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Option<Commands>,

    #[command(flatten)]
    pub run: RunArgs,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Register and manage campaign projects.
    Project(ProjectCli),
    /// Build, list, label, and review speaker voice profiles.
    Profiles(ProfilesCli),
    /// Batch-transcribe episode MP4s (default when no subcommand).
    Run(RunArgs),
    /// Re-transcribe a trimmed clip to Timed words (.words.json) for vod-guru.
    Clip(ClipArgs),
    /// Batch clip STT — one GPU model load for many clips (vod-guru prepare-reviews-batch).
    ClipBatch(ClipBatchArgs),
    /// Detailed command reference (topics: project, profiles, run, models, install).
    Help {
        #[arg(value_name = "TOPIC")]
        topic: Option<String>,
    },
    /// Remove installed CLI, PATH entry, and optional ~/.transcribe data.
    Uninstall {
        /// Also delete ~/.transcribe (project registry).
        #[arg(long)]
        purge: bool,
        /// Skip confirmation prompt.
        #[arg(short, long)]
        yes: bool,
    },
    /// Alias for `profiles` (deprecated).
    #[command(hide = true)]
    Voice(ProfilesCli),
}

#[derive(Parser)]
pub struct ProjectCli {
    #[command(subcommand)]
    pub command: ProjectCommands,
}

#[derive(Subcommand)]
pub enum ProjectCommands {
    /// Register a directory as a named project.
    Init {
        name: String,
        #[arg(long, value_name = "DIR", default_value = ".")]
        path: PathBuf,
        #[arg(long, value_name = "DIR")]
        models_dir: Option<PathBuf>,
    },
    /// List registered projects.
    List,
    /// Set the active project.
    Use { name: String },
    /// Show active project details.
    Show,
}

#[derive(Parser)]
pub struct ProfilesCli {
    #[command(subcommand)]
    pub command: ProfilesCommands,
}

#[derive(Subcommand)]
pub enum ProfilesCommands {
    /// Sample episodes, diarize, and merge global speaker profiles.
    Build {
        #[arg(value_name = "PATH")]
        path: Option<PathBuf>,
        #[arg(long, default_value_t = 18.0)]
        sample_minutes: f64,
        #[arg(long, default_value_t = 8)]
        expected_speakers: i32,
        #[arg(long)]
        force: bool,
        #[arg(long, value_name = "DIR")]
        models_dir: Option<PathBuf>,
        #[arg(long)]
        project: Option<String>,
        #[arg(long, hide = true, default_value = "voices")]
        voices: Option<PathBuf>,
    },
    /// List profiles and clip paths.
    List {
        #[arg(long, hide = true, default_value = "voices")]
        voices: Option<PathBuf>,
    },
    /// Interactively assign display names and colors (plays clips in terminal).
    Label {
        #[arg(long)]
        profile: Option<String>,
        #[arg(long)]
        no_play: bool,
        #[arg(long)]
        project: Option<String>,
    },
    /// Regenerate voices/review.html.
    Review {
        #[arg(long, hide = true, default_value = "voices")]
        voices: Option<PathBuf>,
    },
    /// Non-interactive label (hidden; used internally).
    #[command(hide = true)]
    LabelOne {
        id: String,
        name: String,
        #[arg(long)]
        color: Option<String>,
        #[arg(long, hide = true, default_value = "voices")]
        voices: Option<PathBuf>,
    },
}

#[derive(Parser, Clone)]
pub struct RunArgs {
    /// Video file or directory (default: current working directory).
    #[arg(value_name = "PATH", default_value = ".")]
    pub path: PathBuf,

    #[arg(short, long)]
    pub force: bool,

    #[arg(short, long, default_value = "large-v3")]
    pub model: String,

    #[arg(long, value_name = "DIR")]
    pub models_dir: Option<PathBuf>,

    /// Co-op stream: tag speakers when voice profiles exist (off by default).
    #[arg(long)]
    pub coop: bool,

    #[arg(long)]
    pub no_speakers: bool,

    /// Key terms file (default: `<video>.terms.txt` beside each source MP4).
    #[arg(long, value_name = "FILE")]
    pub terms: Option<PathBuf>,

    #[arg(long)]
    pub project: Option<String>,
}

#[derive(Parser, Clone)]
pub struct ClipArgs {
    /// Trimmed review clip (.mp4 or .wav).
    pub path: PathBuf,

    #[arg(short, long)]
    pub force: bool,

    #[arg(short, long, default_value = "small")]
    pub model: String,

    #[arg(long, value_name = "DIR")]
    pub models_dir: Option<PathBuf>,

    #[arg(long, value_name = "FILE")]
    pub terms: Option<PathBuf>,

    /// Inline key terms (comma-separated); used when --terms is omitted.
    #[arg(long)]
    pub prompt: Option<String>,

    /// Output .words.json path (default: beside clip with .words.json extension).
    #[arg(long, value_name = "FILE")]
    pub output: Option<PathBuf>,
}

#[derive(Parser, Clone)]
pub struct ClipBatchArgs {
    /// JSON job list: [{"clip":"…","output":"…","prompt":"…","force":false}]
    #[arg(long, value_name = "FILE")]
    pub manifest: PathBuf,

    #[arg(short, long, default_value = "small")]
    pub model: String,

    #[arg(long, value_name = "DIR")]
    pub models_dir: Option<PathBuf>,
}

/// Parse argv and run the CLI (used by the binary and tests).
pub fn run_from<I, T>(args: I) -> Result<()>
where
    I: IntoIterator<Item = T>,
    T: Into<OsString> + Clone,
{
    let cli = Cli::parse_from(args);
    dispatch(cli)
}

pub fn run() -> Result<()> {
    dispatch(Cli::parse())
}

fn dispatch(cli: Cli) -> Result<()> {
    match cli.command {
        Some(Commands::Help { topic }) => {
            help::run_help(topic.as_deref());
            Ok(())
        }
        Some(Commands::Uninstall { purge, yes }) => uninstall::run_uninstall(purge, yes),
        Some(Commands::Project(p)) => run_project(p),
        Some(Commands::Profiles(p)) | Some(Commands::Voice(p)) => run_profiles(p),
        Some(Commands::Run(r)) => run_transcribe(r),
        Some(Commands::Clip(c)) => run_clip(c),
        Some(Commands::ClipBatch(b)) => run_clip_batch(b),
        None => run_transcribe(cli.run),
    }
}

fn run_project(p: ProjectCli) -> Result<()> {
    match p.command {
        ProjectCommands::Init {
            name,
            path,
            models_dir,
        } => crate::project::run_init(&name, &path, models_dir.as_deref()),
        ProjectCommands::List => crate::project::run_list(),
        ProjectCommands::Use { name } => crate::project::run_use(&name),
        ProjectCommands::Show => crate::project::run_show(),
    }
}

fn run_profiles(p: ProfilesCli) -> Result<()> {
    match p.command {
        ProfilesCommands::Build {
            path,
            sample_minutes,
            expected_speakers,
            force,
            models_dir,
            project,
            voices: _,
        } => run_profiles_build(
            path.as_deref(),
            project.as_deref(),
            models_dir,
            sample_minutes,
            expected_speakers,
            force,
        ),
        ProfilesCommands::List { voices } => {
            let voices = resolve_voices_dir(voices.as_deref(), None)?;
            voice::run_list(&voices)
        }
        ProfilesCommands::Label {
            profile,
            no_play,
            project,
        } => label::run_interactive(project.as_deref(), profile.as_deref(), no_play),
        ProfilesCommands::Review { voices } => {
            let voices = resolve_voices_dir(voices.as_deref(), None)?;
            voice::run_review(&voices)
        }
        ProfilesCommands::LabelOne {
            id,
            name,
            color,
            voices,
        } => {
            let voices = resolve_voices_dir(voices.as_deref(), None)?;
            voice::run_label(&voices, &id, &name, color.as_deref())
        }
    }
}

fn run_profiles_build(
    path: Option<&Path>,
    project_name: Option<&str>,
    models_dir: Option<PathBuf>,
    sample_minutes: f64,
    expected_speakers: i32,
    force: bool,
) -> Result<()> {
    let db = open_db()?;
    let project = require_project(&db, project_name)?;
    let scan_path = crate::paths::resolve_working_path(path)?;
    let voices = project.voices_path();
    let models = resolve_models_dir(Some(&project), models_dir)?;
    eprintln!("models: {}", models.display());

    let project_id = project.id;
    let mut checkpoint =
        |stem: &str| -> Result<()> { mark_episode_profile_built(&db, project_id, stem) };

    voice::run_build(
        &scan_path,
        &voices,
        &models,
        sample_minutes,
        expected_speakers,
        force,
        Some(&mut checkpoint),
    )?;

    sync_profiles(&db, &project)?;
    sync_episodes(&db, &project, &scan_path)?;
    eprintln!("synced profiles and episodes to registry");
    Ok(())
}

fn resolve_voices_dir(cli_voices: Option<&Path>, project_name: Option<&str>) -> Result<PathBuf> {
    if let Some(v) = cli_voices {
        return Ok(v.to_path_buf());
    }
    let db = open_db()?;
    let project = require_project(&db, project_name)?;
    Ok(project.voices_path())
}

pub fn run_transcribe(cli: RunArgs) -> Result<()> {
    let db = open_db()?;
    let project = require_project(&db, cli.project.as_deref()).ok();

    let scan_path = crate::paths::resolve_working_path(Some(&cli.path))?;
    eprintln!("source: {}", scan_path.display());

    let videos = discover_videos(&scan_path)?;
    if videos.is_empty() {
        bail!("no .mp4 files found under {}", scan_path.display());
    }

    let models_dir = resolve_models_dir(project.as_ref(), cli.models_dir.clone())?;
    eprintln!("models: {}", models_dir.display());
    let voices_dir = project
        .as_ref()
        .map(|p| p.voices_path())
        .unwrap_or_else(|| std::env::current_dir().unwrap_or_default().join("voices"));

    let model_path = models_dir.join(format!("ggml-{}.bin", cli.model));
    if !model_path.is_file() {
        bail!(
            "model not found: {}\nDownload ggml-{}.bin into {}",
            model_path.display(),
            cli.model,
            models_dir.display()
        );
    }

    if let Some(ref p) = project {
        sync_episodes(&db, p, &scan_path)?;
    }

    let use_speakers = cli.coop
        && !cli.no_speakers
        && voices_dir.join(voice::profiles::PROFILES_FILE).is_file();
    let mut voice_engine = if use_speakers {
        eprintln!("speaker tagging: {}", voices_dir.display());
        Some(VoiceEngine::for_transcribe(&models_dir, &voices_dir)?)
    } else {
        if cli.coop && !cli.no_speakers && project.is_some() {
            eprintln!("warning: --coop set but no profiles.json; transcribing without speakers");
        } else if cli.coop && !cli.no_speakers && project.is_none() {
            eprintln!("warning: --coop requires an active project with profiles; transcribing without speakers");
        }
        None
    };

    eprintln!("loading model: {}", model_path.display());
    log_whisper_backend();
    let ctx = WhisperContext::new_with_params(
        model_path.to_str().context("model path is not UTF-8")?,
        whisper_context_params(),
    )
    .context("load whisper model (CUDA build needs toolkit + driver)")?;

    let vad_path = find_vad_model(&models_dir);
    if let Some(ref vad) = vad_path {
        eprintln!("VAD enabled: {}", vad.display());
    }

    let project_id = project.as_ref().map(|p| p.id);
    let mut had_errors = false;
    for video in videos {
        match process_one(
            &ctx,
            &video,
            cli.force,
            vad_path.as_deref(),
            voice_engine.as_mut(),
            use_speakers,
            cli.terms.as_deref(),
        ) {
            Ok(()) => {
                if let Some(pid) = project_id {
                    let _ = mark_episode_transcribed(&db, pid, &video);
                }
            }
            Err(e) => {
                eprintln!("error: {}: {:#}", video.display(), e);
                if let Some(pid) = project_id {
                    let _ = mark_episode_error(&db, pid, &video, &format!("{e:#}"));
                }
                had_errors = true;
            }
        }
    }

    if had_errors {
        bail!("one or more episodes failed");
    }
    Ok(())
}

pub fn run_clip(cli: ClipArgs) -> Result<()> {
    let models_dir = resolve_models_dir(None, cli.models_dir.clone())?;
    eprintln!("models: {}", models_dir.display());
    log_whisper_backend();

    let ctx = load_whisper_context(&models_dir, &cli.model)?;
    let vad_path = find_vad_model(&models_dir);
    let output = cli
        .output
        .clone()
        .unwrap_or_else(|| cli.path.with_extension("words.json"));
    let initial_prompt = cli
        .terms
        .as_ref()
        .and_then(|path| load_initial_prompt(path))
        .or(cli.prompt.clone());

    process_clip_with_ctx(
        &ctx,
        &cli.path,
        &output,
        initial_prompt.as_deref(),
        vad_path.as_deref(),
        cli.force,
    )
}

#[derive(Debug, Deserialize)]
struct ClipBatchJob {
    clip: PathBuf,
    output: PathBuf,
    #[serde(default)]
    prompt: Option<String>,
    #[serde(default)]
    force: bool,
}

pub fn run_clip_batch(cli: ClipBatchArgs) -> Result<()> {
    let manifest_raw = std::fs::read_to_string(&cli.manifest)
        .with_context(|| format!("read manifest {}", cli.manifest.display()))?;
    let jobs: Vec<ClipBatchJob> =
        serde_json::from_str(&manifest_raw).context("parse clip-batch manifest JSON")?;
    if jobs.is_empty() {
        bail!("clip-batch manifest is empty");
    }

    let models_dir = resolve_models_dir(None, cli.models_dir.clone())?;
    eprintln!("models: {}", models_dir.display());
    eprintln!("clip-batch: {} job(s)", jobs.len());
    log_whisper_backend();

    let ctx = load_whisper_context(&models_dir, &cli.model)?;
    let vad_path = find_vad_model(&models_dir);

    let mut had_errors = false;
    for (index, job) in jobs.iter().enumerate() {
        eprintln!("clip-batch [{}/{}]", index + 1, jobs.len());
        if let Err(error) = process_clip_with_ctx(
            &ctx,
            &job.clip,
            &job.output,
            job.prompt.as_deref(),
            vad_path.as_deref(),
            job.force,
        ) {
            eprintln!("error: {}: {:#}", job.clip.display(), error);
            had_errors = true;
        }
    }

    if had_errors {
        bail!("one or more clip-batch jobs failed");
    }
    Ok(())
}

fn load_whisper_context(models_dir: &Path, model: &str) -> Result<WhisperContext> {
    let model_path = models_dir.join(format!("ggml-{model}.bin"));
    if !model_path.is_file() {
        bail!(
            "model not found: {}\nDownload ggml-{model}.bin into {}",
            model_path.display(),
            models_dir.display()
        );
    }
    eprintln!("loading model: {}", model_path.display());
    WhisperContext::new_with_params(
        model_path.to_str().context("model path is not UTF-8")?,
        whisper_context_params(),
    )
    .context("load whisper model (CUDA build needs toolkit + driver)")
}

fn process_clip_with_ctx(
    ctx: &WhisperContext,
    clip: &Path,
    output: &Path,
    initial_prompt: Option<&str>,
    vad_model: Option<&Path>,
    force: bool,
) -> Result<()> {
    if !clip.is_file() {
        bail!("clip not found: {}", clip.display());
    }
    if output.is_file() && !force {
        eprintln!("skipped: {}", output.display());
        return Ok(());
    }

    eprintln!("clip STT: {} -> {}", clip.display(), output.display());
    if let Some(prompt) = initial_prompt {
        if !prompt.is_empty() {
            eprintln!("key terms: {prompt}");
        }
    }

    let (samples, wav_path) = audio::extract_and_load(clip)?;
    let result = transcribe(ctx, &samples, vad_model, initial_prompt)?;
    audio::remove_temp_wav(&wav_path);
    write_words_json(output, &result.words)?;
    eprintln!("done: {}", output.display());
    Ok(())
}

pub fn find_vad_model(models_dir: &Path) -> Option<PathBuf> {
    for name in ["ggml-silero-v6.2.0.bin", "ggml-silero-v5.1.2.bin"] {
        let path = models_dir.join(name);
        if path.is_file() {
            return Some(path);
        }
    }
    None
}

pub fn outputs_exist(video: &Path, with_speakers: bool) -> bool {
    let stem = video.with_extension("");
    let txt = stem.with_extension("txt").is_file();
    let srt = stem.with_extension("srt").is_file();
    let words = stem.with_extension("words.json").is_file();
    if with_speakers {
        txt && srt && words && stem.with_extension("ass").is_file()
    } else {
        txt && srt && words
    }
}

fn resolve_terms_path(video: &Path, cli_terms: Option<&Path>) -> PathBuf {
    cli_terms
        .map(Path::to_path_buf)
        .unwrap_or_else(|| video.with_extension("terms.txt"))
}

pub struct TranscribeOutput {
    pub segments: Vec<Segment>,
    pub words: Vec<TimedWord>,
}

pub fn process_one(
    ctx: &WhisperContext,
    video: &Path,
    force: bool,
    vad_model: Option<&Path>,
    voice_engine: Option<&mut VoiceEngine>,
    with_speakers: bool,
    terms_path: Option<&Path>,
) -> Result<()> {
    let label = video
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("episode");

    if !force && outputs_exist(video, with_speakers) {
        eprintln!("skipped: {label}");
        return Ok(());
    }

    eprintln!("transcribing: {label}...");
    let (samples, wav_path) = audio::extract_and_load(video)?;
    let terms = resolve_terms_path(video, terms_path);
    let initial_prompt = load_initial_prompt(&terms);
    if let Some(ref prompt) = initial_prompt {
        eprintln!("key terms: {} (from {})", prompt, terms.display());
    }
    let mut output = transcribe(ctx, &samples, vad_model, initial_prompt.as_deref())?;

    if let Some(engine) = voice_engine.as_ref() {
        for seg in &mut output.segments {
            seg.speaker = engine.identify_range(&samples, seg.start_cs, seg.end_cs);
        }
    }
    apply_word_speakers(&mut output.words, &output.segments);

    audio::remove_temp_wav(&wav_path);

    let stem = video.with_extension("");
    let txt_path = stem.with_extension("txt");
    let srt_path = stem.with_extension("srt");
    let words_path = stem.with_extension("words.json");
    write_txt(&txt_path, label, &output.segments)?;
    write_srt(&srt_path, &output.segments)?;
    write_words_json(&words_path, &output.words)?;

    if with_speakers {
        if let Some(engine) = voice_engine.as_ref() {
            let ass_path = stem.with_extension("ass");
            write_ass(&ass_path, &output.segments, &engine.store)?;
        }
    }

    eprintln!("done: {label}");
    Ok(())
}

pub fn transcribe(
    ctx: &WhisperContext,
    samples: &[f32],
    vad_model: Option<&Path>,
    initial_prompt: Option<&str>,
) -> Result<TranscribeOutput> {
    let mut state = ctx.create_state().context("create whisper state")?;
    let mut params = FullParams::new(SamplingStrategy::Greedy { best_of: 1 });

    params.set_language(Some("en"));
    params.set_translate(false);
    params.set_print_special(false);
    params.set_print_progress(false);
    params.set_print_realtime(false);
    params.set_print_timestamps(false);
    params.set_token_timestamps(true);
    if let Some(prompt) = initial_prompt {
        if !prompt.is_empty() {
            params.set_initial_prompt(prompt);
        }
    }

    if let Some(vad) = vad_model {
        params.set_vad_model_path(Some(vad.to_str().context("VAD model path is not UTF-8")?));
        params.enable_vad(true);
    }

    state
        .full(params, samples)
        .context("whisper inference failed")?;

    let segments = collect_segments(&state)?;
    let words = collect_words(&state)?;
    Ok(TranscribeOutput { segments, words })
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn outputs_exist_checks_extensions() {
        let tmp = TempDir::new().unwrap();
        let video = tmp.path().join("ep.mp4");
        std::fs::write(&video, b"x").unwrap();
        let stem = video.with_extension("");
        assert!(!outputs_exist(&video, false));
        std::fs::write(stem.with_extension("txt"), "t").unwrap();
        assert!(!outputs_exist(&video, false));
        std::fs::write(stem.with_extension("srt"), "s").unwrap();
        assert!(!outputs_exist(&video, false));
        std::fs::write(stem.with_extension("words.json"), "[]").unwrap();
        assert!(outputs_exist(&video, false));
        assert!(!outputs_exist(&video, true));
        std::fs::write(stem.with_extension("ass"), "a").unwrap();
        assert!(outputs_exist(&video, true));
    }

    #[test]
    fn find_vad_model_none_when_missing() {
        let tmp = TempDir::new().unwrap();
        assert!(find_vad_model(tmp.path()).is_none());
    }
}
