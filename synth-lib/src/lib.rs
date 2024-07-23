#![feature(trivial_bounds)]
use anyhow::Result;
use audio::{AudioOutputSync, TrackerSynth};
use rodio::{OutputStream, OutputStreamHandle, Source};
use std::sync::{Arc, Mutex};
pub use synth_8080::start_logging;
pub use synth_8080_lib::notes::Note;
use tracing::*;

pub mod audio;
pub mod synth;

pub fn init_synth() -> Result<(
    Arc<Mutex<TrackerSynth>>,
    OutputStreamHandle,
    impl Source<Item = f32> + Iterator<Item = f32>,
)> {
    start_logging()?;
    info!("initializing synth");

    // let (sample_dest, sample_rx) = unbounded();
    // let (sync_tx, sync) = unbounded();

    let synth = Arc::new(Mutex::new(TrackerSynth::default()));
    let (_stream, stream_handle) = OutputStream::try_default().unwrap();

    // let a_io = audio::AudioOutput {
    //     synth,
    //     sample_dest,
    //     _stream,
    //     sync,
    // };
    //
    // let audio = Audio::new(sync_tx, sample_rx);
    let audio = AudioOutputSync {
        synth: synth.clone(),
        _stream,
    };

    Ok((synth, stream_handle, audio))
}

// #[cfg(test)]
// mod tests {
//     use super::*;
//
//     // #[test]
//     // fn it_works() {
//     //     let result = add(2, 2);
//     //     assert_eq!(result, 4);
//     // }
// }
