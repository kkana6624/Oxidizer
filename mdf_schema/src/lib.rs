use serde::{Deserialize, Serialize};
use std::collections::HashMap;

pub type Microseconds = u64;

#[derive(Debug, Serialize, Deserialize, PartialEq)]
pub struct MdfChart {
    pub meta: Metadata,
    #[serde(default)]
    pub resources: HashMap<String, String>,
    pub visual_events: Vec<VisualEvent>,
    pub speed_events: Vec<SpeedEvent>,
    pub notes: Vec<Note>,
    pub bgm_events: Vec<BgmEvent>,
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
pub struct Metadata {
    pub title: String,
    pub artist: String,
    pub version: String,
    pub total_duration_us: Microseconds,
    pub tags: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
pub struct VisualEvent {
    pub time_us: Microseconds,
    pub bpm: f64,
    pub is_measure_line: bool,
    pub beat_n: u32,
    pub beat_d: u32,
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
pub struct SpeedEvent {
    pub time_us: Microseconds,
    pub scroll_rate: f64,
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
pub struct Note {
    pub time_us: Microseconds,
    pub col: u8,
    #[serde(flatten)]
    pub kind: NoteKind,
    pub sound_id: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type")]
pub enum NoteKind {
    #[serde(rename = "tap")]
    Tap,

    #[serde(rename = "cn")]
    ChargeNote { end_time_us: Microseconds },

    #[serde(rename = "hcn")]
    HellChargeNote { end_time_us: Microseconds },

    #[serde(rename = "bss")]
    BackSpinScratch { end_time_us: Microseconds },

    #[serde(rename = "hbss")]
    HellBackSpinScratch { end_time_us: Microseconds },

    #[serde(rename = "mss")]
    MultiSpinScratch {
        end_time_us: Microseconds,
        #[serde(default)]
        reverse_checkpoints_us: Vec<Microseconds>,
    },

    #[serde(rename = "hmss")]
    HellMultiSpinScratch {
        end_time_us: Microseconds,
        #[serde(default)]
        reverse_checkpoints_us: Vec<Microseconds>,
    },
}

impl NoteKind {
    pub fn end_time_us(&self) -> Option<Microseconds> {
        match self {
            NoteKind::Tap => None,
            NoteKind::ChargeNote { end_time_us }
            | NoteKind::HellChargeNote { end_time_us }
            | NoteKind::BackSpinScratch { end_time_us }
            | NoteKind::HellBackSpinScratch { end_time_us }
            | NoteKind::MultiSpinScratch { end_time_us, .. }
            | NoteKind::HellMultiSpinScratch { end_time_us, .. } => Some(*end_time_us),
        }
    }
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
pub struct BgmEvent {
    pub time_us: Microseconds,
    pub sound_id: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn note_kind_serialization_includes_type_tag() {
        let note = Note {
            time_us: 123,
            col: 3,
            kind: NoteKind::ChargeNote { end_time_us: 456 },
            sound_id: Some("K01".to_string()),
        };

        let json = serde_json::to_value(&note).unwrap();
        assert_eq!(json["type"], "cn");
        assert_eq!(json["end_time_us"], 456);
        assert_eq!(json["time_us"], 123);
        assert_eq!(json["col"], 3);
        assert_eq!(json["sound_id"], "K01");
    }

    #[test]
    fn mss_reverse_checkpoints_default_empty() {
        let v = serde_json::json!({
            "time_us": 0,
            "col": 0,
            "type": "mss",
            "end_time_us": 400000,
            "sound_id": "S_MS"
        });

        let note: Note = serde_json::from_value(v).unwrap();
        match note.kind {
            NoteKind::MultiSpinScratch {
                end_time_us,
                reverse_checkpoints_us,
            } => {
                assert_eq!(end_time_us, 400000);
                assert!(reverse_checkpoints_us.is_empty());
            }
            _ => panic!("unexpected kind"),
        }
    }

    #[test]
    fn chart_roundtrip_minimal() {
        let mut resources = HashMap::new();
        resources.insert("K01".to_string(), "kick.wav".to_string());

        let chart = MdfChart {
            meta: Metadata {
                title: "t".to_string(),
                artist: "a".to_string(),
                version: "2.2".to_string(),
                total_duration_us: 500,
                tags: vec!["training".to_string()],
            },
            resources,
            visual_events: vec![],
            speed_events: vec![],
            notes: vec![Note {
                time_us: 0,
                col: 1,
                kind: NoteKind::Tap,
                sound_id: Some("K01".to_string()),
            }],
            bgm_events: vec![BgmEvent {
                time_us: 500,
                sound_id: "SE_END".to_string(),
            }],
        };

        let json = serde_json::to_string(&chart).unwrap();
        let back: MdfChart = serde_json::from_str(&json).unwrap();
        assert_eq!(chart, back);
    }
}

