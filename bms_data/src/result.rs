use serde::{Serialize, Deserialize};
use crate::model::Lane;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum RandomMode {
    Normal,
    Mirror,
    Random,
    SRandom,
    RRandom,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Judge {
    PGreat,
    Great,
    Good,
    Bad,
    Poor,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HitEvent {
    pub note_index: usize,
    pub original_tick: u32,
    pub lane: Lane,
    pub judge: Judge,
    pub delta_ms: Option<f64>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PlayResult {
    pub chart_checksum: String,
    pub random_mode: RandomMode,
    pub events: Vec<HitEvent>,
    pub timestamp: u64,
}
