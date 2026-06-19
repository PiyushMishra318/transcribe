//! Episode Transcribe library — batch MP4 transcriber (whisper.cpp + sherpa-onnx).

pub mod audio;
pub mod cli;
pub mod db;
pub mod gpu;
pub mod help;
pub mod label;
pub mod output;
pub mod paths;
pub mod terms;
pub mod project;
pub mod uninstall;
pub mod voice;

#[cfg(test)]
mod test_util;
