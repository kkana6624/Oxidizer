pub mod model;
pub mod result;
pub mod profile;

#[cfg(test)]
mod tests {
    use super::model::*;
    use std::collections::HashMap;

    #[test]
    fn test_chart_serialization() {
        let mut wav_files = HashMap::new();
        wav_files.insert(1, "sound1.wav".to_string());

        let header = Header {
            title: "Test Song".to_string(),
            artist: "Test Artist".to_string(),
            initial_bpm: 150.0,
            generation_seed: Some(123456789),
            wav_files,
        };

        let note = Note {
            tick: 480,
            lane: Lane::Key1,
            kind: NoteKind::Normal,
            sound_id: 1,
        };

        let chart = Chart {
            header,
            notes: vec![note],
            bar_lines: vec![0, 1920],
            bpm_changes: vec![(0, 150.0)],
            patterns: vec![],
        };

        let json = serde_json::to_string(&chart).expect("Failed to serialize chart");
        println!("Serialized Chart: {}", json);

        let deserialized_chart: Chart = serde_json::from_str(&json).expect("Failed to deserialize chart");

        assert_eq!(deserialized_chart.header.title, "Test Song");
        assert_eq!(deserialized_chart.notes.len(), 1);
        assert_eq!(deserialized_chart.notes[0].tick, 480);
        assert_eq!(deserialized_chart.notes[0].lane, Lane::Key1);
    }
}
