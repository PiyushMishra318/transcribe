//! Episode Transcribe binary entrypoint.

fn main() -> anyhow::Result<()> {
    episode_transcribe::cli::run()
}
