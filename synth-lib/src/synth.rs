use anyhow::Result;
use synth_8080::{common::Module, midi_osc::MidiOsc, Float};
use synth_8080_lib::{notes::Note, OscType};

pub struct Synth {
    pub synth_num: usize,
    pub name: String,
    pub vol: Float,
    synth: MidiOsc,
}

impl Synth {
    pub fn new(synth_number: usize) -> Self {
        let name = format!("{synth_number}");
        let synth = MidiOsc::new(4);

        Self {
            synth_num: synth_number,
            name,
            vol: 1.0,
            synth,
        }
    }

    pub fn get_sample(&mut self) -> Float {
        self.synth.get_samples()[0].1
    }

    pub fn play_note(&mut self, note: Note) -> Result<()> {
        if !self.synth.is_playing(note) {
            self.synth.play_note(note)?;
        } else {
            self.stop_note(note)?;
        }

        Ok(())
    }

    pub fn stop_note(&mut self, note: Note) -> Result<()> {
        self.synth.stop_note(note)?;

        Ok(())
    }

    pub fn set_waveform(&mut self, waveform: OscType) {
        self.synth.set_wave_form(waveform);
    }
}
