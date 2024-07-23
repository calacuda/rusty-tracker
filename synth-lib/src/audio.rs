use super::synth::Synth;
use anyhow::{bail, Result};
use rayon::prelude::*;
use rodio::{OutputStream, Source};
use std::sync::{Arc, Mutex};
use synth_8080::{Float, SAMPLE_RATE};
use synth_8080_lib::notes::Note;
use tracing::*;
use tracker_lib::ChannelIndex;

// /// an async struct meant to handle the syntronization of audio sample generation and output. it
// /// will have the synth synthisize a sample then it will send that sample to the output struct.
// pub struct AudioOutput {
//     pub synth: Arc<Mutex<TrackerSynth>>,
//     pub sample_dest: Sender<Float>,
//     /// the rodio output stream, it isn't used but must never be dropped else audio output will cease
//     pub _stream: OutputStream,
//     pub sync: Receiver<()>,
// }
//
// impl Future for AudioOutput {
//     type Output = ();
//
//     fn poll(self: std::pin::Pin<&mut Self>, cx: &mut std::task::Context<'_>) -> Poll<Self::Output> {
//         // self.deref().controller.step();
//         // info!("waiting on sync signal");
//
//         if let Err(_e) = self.sync.try_recv() {
//             // error!("error receiving sync message: {e}");
//             cx.waker().wake_by_ref();
//             return Poll::Pending;
//         };
//
//         let sample = self.synth.lock().unwrap().get_sample();
//
//         if let Err(e) = self.sample_dest.send(sample) {
//             error!("sending sample to output struct failed with error: {e}");
//         }
//
//         cx.waker().wake_by_ref();
//         Poll::Pending
//     }
// }
//
// unsafe impl Send for AudioOutput {}

pub struct AudioOutputSync {
    pub synth: Arc<Mutex<TrackerSynth>>,
    pub _stream: OutputStream,
}

impl Iterator for AudioOutputSync {
    type Item = f32;

    fn next(&mut self) -> Option<Self::Item> {
        Some(self.synth.lock().unwrap().get_sample() as f32)
    }
}

impl Source for AudioOutputSync {
    fn current_frame_len(&self) -> Option<usize> {
        None
    }

    fn channels(&self) -> u16 {
        1
    }

    fn sample_rate(&self) -> u32 {
        // 48_000
        // 44_100
        // 22_050
        SAMPLE_RATE
    }

    fn total_duration(&self) -> Option<std::time::Duration> {
        None
    }
}

unsafe impl Send for AudioOutputSync {}

pub struct TrackerSynth {
    pub synths: Vec<Synth>,
    discount: Float,
}

impl Default for TrackerSynth {
    fn default() -> Self {
        Self::new(2)
    }
}

impl TrackerSynth {
    /// n represents how many synths should be started
    pub fn new(n: usize) -> Self {
        let synths = (0..n).into_iter().map(|i| Synth::new(i)).collect();
        let discount = 1.0 / (n as Float);

        Self { synths, discount }
    }

    pub fn get_sample(&mut self) -> Float {
        let sample: Float = self
            .synths
            .par_iter_mut()
            .map(|synth| synth.get_sample())
            .sum();

        (sample * self.discount).tanh()
    }

    fn channel_len_check(&mut self, channel: usize) -> Result<()> {
        if channel >= self.synths.len() {
            let mesg = format!("the channel {channel} does not exist.");
            error!(mesg);
            bail!(mesg);
        }

        Ok(())
    }

    pub fn play(&mut self, note: Note, channel: usize) -> Result<()> {
        self.channel_len_check(channel)?;

        if let Err(e) = self.synths[channel].play_note(note) {
            let mesg = format!("playing \"{note}\" on channel {channel}, resulted in error: {e}");
            error!(mesg);
            bail!(mesg);
        }

        Ok(())
    }

    pub fn stop(&mut self, note: Note, channel: usize) -> Result<()> {
        self.channel_len_check(channel)?;

        if let Err(e) = self.synths[channel].stop_note(note) {
            let mesg = format!("playing \"{note}\" on channel {channel}, resulted in error: {e}");
            error!(mesg);
            bail!(mesg);
        }

        Ok(())
    }

    // pub fn set_volume(&mut self, volume: Float, channel: ChannelIndex) -> Result<()> {
    //     let channel = channel as usize;
    //
    //     if channel < self.synths.len() {
    //         self.synths[channel].vol = volume;
    //         Ok(())
    //     } else {
    //         let msg = "can't set the volume of channel \"{channel}\" because, that channel doesn't exist does not exist.";
    //         error!(msg);
    //         bail!(msg);
    //     }
    // }

    pub fn set_volume(&mut self, volume: Float, channel: ChannelIndex) -> Result<()> {
        let channel = channel as usize;

        self.channel_len_check(channel)?;
        self.synths[channel].vol = volume;

        Ok(())
    }
}
