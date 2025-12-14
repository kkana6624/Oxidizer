use std::collections::HashMap;
use serde::{Serialize, Deserialize};

pub const TICKS_PER_BEAT: u32 = 480;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Lane {
    Scratch,
    Key1,
    Key2,
    Key3,
    Key4,
    Key5,
    Key6,
    Key7,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum NoteKind {
    Normal,
    LongStart,
    LongEnd,
    Mine,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Note {
    pub tick: u32,
    pub lane: Lane,
    pub kind: NoteKind,
    pub sound_id: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum PatternType {
    Trill,
    Stair,
    Chord,
    Denim,
    Jack,
    ScratchComplex,
    Generic,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PatternTag {
    pub start_tick: u32,
    pub end_tick: u32,
    pub pattern: PatternType,
    pub intensity: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Header {
    pub title: String,
    pub artist: String,
    pub initial_bpm: f64,
    pub generation_seed: Option<u64>,
    pub wav_files: HashMap<u32, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Chart {
    pub header: Header,
    pub notes: Vec<Note>,
    pub bar_lines: Vec<u32>,
    pub bpm_changes: Vec<(u32, f64)>,
    pub patterns: Vec<PatternTag>,
}
