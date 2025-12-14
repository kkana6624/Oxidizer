#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum NoteKind {
    Normal,
    ChargeStart,
    ChargeEnd,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Judgment {
    Perfect,
    Great,
    Good,
    Bad,
    Poor,
    Miss,
}

#[derive(Debug, Clone)]
pub struct Note {
    pub time: f64,
    pub lane: usize, // 1-7, Scratch=0
    pub kind: NoteKind,
    pub sound_id: Option<usize>,
}

#[derive(Debug, Clone)]
pub struct BpmChange {
    pub time: f64,
    pub bpm: f64,
}

#[derive(Debug, Clone, Default)]
pub struct Chart {
    pub notes: Vec<Note>,
    pub bpm_changes: Vec<BpmChange>,
}

impl Chart {
    pub fn dummy() -> Self {
        let mut notes = Vec::new();
        // Simple 4-beat rhythm at 120 BPM
        // Beat 1: 0.0s, Lane 1
        // Beat 2: 0.5s, Lane 2
        // Beat 3: 1.0s, Lane 3
        // Beat 4: 1.5s, Lane 4
        // Beat 5: 2.0s, Lane 1 (Simultaneous with Scratch?)

        for i in 0..8 {
            notes.push(Note {
                time: i as f64 * 0.5,
                lane: (i % 7) + 1, // 1 to 7 cycle
                kind: NoteKind::Normal,
                sound_id: None,
            });
        }
        // Add a scratch note
        notes.push(Note {
            time: 2.0,
            lane: 0,
            kind: NoteKind::Normal,
            sound_id: None,
        });

        // Sort by time just in case, though here it's sorted
        notes.sort_by(|a, b| a.time.partial_cmp(&b.time).unwrap());

        Chart {
            notes,
            bpm_changes: vec![BpmChange { time: 0.0, bpm: 120.0 }],
        }
    }
}
