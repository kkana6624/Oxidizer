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

        assert_eq!(deserialized_chart.header.title, "Test Song");
        assert_eq!(deserialized_chart.notes.len(), 1);
        assert_eq!(deserialized_chart.notes[0].tick, 480);
        assert_eq!(deserialized_chart.notes[0].lane, Lane::Key1);
    }

    #[test]
    fn test_result_serialization() {
        let hit_event = HitEvent {
            note_index: 0,
            original_tick: 480,
            lane: Lane::Key1,
            judge: Judge::PGreat,
            delta_ms: Some(5.5),
        };

        let play_result = PlayResult {
            chart_checksum: "abc123".to_string(),
            random_mode: RandomMode::Normal,
            events: vec![hit_event],
            timestamp: 1234567890,
        };

        let json = serde_json::to_string(&play_result).expect("Failed to serialize play result");
        println!("Serialized PlayResult: {}", json);

        let deserialized_result: PlayResult = serde_json::from_str(&json).expect("Failed to deserialize play result");

        assert_eq!(deserialized_result.chart_checksum, "abc123");
        assert_eq!(deserialized_result.random_mode, RandomMode::Normal);
        assert_eq!(deserialized_result.events.len(), 1);
        assert_eq!(deserialized_result.events[0].note_index, 0);
        assert_eq!(deserialized_result.events[0].judge, Judge::PGreat);
    }

    #[test]
    fn test_profile_serialization() {
        let pattern_stats = PatternStats {
            total_notes: 100,
            miss_count: 5,
            avg_delta_ms: 3.2,
            delta_variance: 1.5,
        };

        let lane_stats = LaneStats {
            total_notes: 50,
            miss_count: 2,
            avg_delta_ms: 2.8,
        };

        let mut pattern_stats_map = HashMap::new();
        pattern_stats_map.insert(PatternType::Trill, pattern_stats);

        let mut lane_stats_map = HashMap::new();
        lane_stats_map.insert(Lane::Key1, lane_stats);

        let user_profile = UserProfile {
            user_name: "TestUser".to_string(),
            pattern_stats: pattern_stats_map,
            lane_stats: lane_stats_map,
            last_updated: 1234567890,
        };

        let json = serde_json::to_string(&user_profile).expect("Failed to serialize user profile");
        println!("Serialized UserProfile: {}", json);

        let deserialized_profile: UserProfile = serde_json::from_str(&json).expect("Failed to deserialize user profile");

        assert_eq!(deserialized_profile.user_name, "TestUser");
        assert_eq!(deserialized_profile.last_updated, 1234567890);
        assert_eq!(deserialized_profile.pattern_stats.len(), 1);
        assert_eq!(deserialized_profile.lane_stats.len(), 1);
    }
}
