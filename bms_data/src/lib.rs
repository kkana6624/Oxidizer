pub mod model;
pub mod result;
pub mod profile;

#[cfg(test)]
mod tests {
    use super::model::*;
    use super::result::*;
    use super::profile::*;
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

        assert_eq!(deserialized_chart, chart);
    }

    #[test]
    fn test_result_serialization() {
        let event = HitEvent {
            note_index: 0,
            original_tick: 480,
            lane: Lane::Key1,
            judge: Judge::PGreat,
            delta_ms: Some(5.5),
        };

        let result = PlayResult {
            chart_checksum: "checksum".to_string(),
            random_mode: RandomMode::Normal,
            events: vec![event],
            timestamp: 1234567890,
        };

        let json = serde_json::to_string(&result).expect("Failed to serialize result");
        let deserialized_result: PlayResult = serde_json::from_str(&json).expect("Failed to deserialize result");

        assert_eq!(deserialized_result, result);
    }

    #[test]
    fn test_profile_serialization() {
        let mut pattern_stats = HashMap::new();
        pattern_stats.insert(PatternType::Trill, PatternStats {
            total_notes: 100,
            miss_count: 5,
            avg_delta_ms: 10.0,
            delta_variance: 2.0,
        });

        let mut lane_stats = HashMap::new();
        lane_stats.insert(Lane::Key1, LaneStats {
            total_notes: 50,
            miss_count: 2,
            avg_delta_ms: 8.0,
        });

        let profile = UserProfile {
            user_name: "Player1".to_string(),
            pattern_stats,
            lane_stats,
            last_updated: 1234567890,
        };

        let json = serde_json::to_string(&profile).expect("Failed to serialize profile");
        let deserialized_profile: UserProfile = serde_json::from_str(&json).expect("Failed to deserialize profile");

        assert_eq!(deserialized_profile, profile);
    }
}
