#[cfg(test)]
mod tests {
    use oxidizer_core::chart::{Chart, Note, NoteKind, Judgment};
    use oxidizer_core::gameplay::judge::{JudgeMachine, JudgmentResult};
    use oxidizer_core::input::events::{Button, InputEvent};

    fn create_test_chart() -> Chart {
        // Create a simple chart:
        // Note 0: Time 1.0, Lane 1
        // Note 1: Time 2.0, Lane 1
        // Note 2: Time 1.5, Lane 2
        let notes = vec![
            Note { time: 1.0, lane: 1, kind: NoteKind::Normal, sound_id: None }, // Index 0
            Note { time: 1.5, lane: 2, kind: NoteKind::Normal, sound_id: None }, // Index 1
            Note { time: 2.0, lane: 1, kind: NoteKind::Normal, sound_id: None }, // Index 2
        ];
        Chart {
            notes,
            bpm_changes: vec![],
        }
    }

    #[test]
    fn test_perfect_judgment() {
        let chart = create_test_chart();
        let mut judge = JudgeMachine::new();

        // Perfect hit for Note 0 (Time 1.0)
        let event = InputEvent {
            timestamp: 1.0,
            button: Button::Key1,
            pressed: true,
        };

        let result = judge.process_input(event, &chart).expect("Should trigger judgment");

        assert_eq!(result.judgment, Judgment::Perfect);
        assert_eq!(result.note_index, 0);
        assert_eq!(result.delta, 0.0);
    }

    #[test]
    fn test_late_good_judgment() {
        let chart = create_test_chart();
        let mut judge = JudgeMachine::new();

        // Note 0 is at 1.0.
        // Good window is 0.100.
        // Hit at 1.08 -> +0.08 -> Good (Late)
        let event = InputEvent {
            timestamp: 1.08,
            button: Button::Key1,
            pressed: true,
        };

        let result = judge.process_input(event, &chart).expect("Should trigger judgment");
        assert_eq!(result.judgment, Judgment::Good);
        assert_eq!(result.note_index, 0);
    }

    #[test]
    fn test_early_bad_judgment() {
        let chart = create_test_chart();
        let mut judge = JudgeMachine::new();

        // Note 0 at 1.0.
        // Bad window is 0.200. Good is 0.100.
        // Hit at 0.85 -> -0.15 -> Bad (Early)
        let event = InputEvent {
            timestamp: 0.85,
            button: Button::Key1,
            pressed: true,
        };

        let result = judge.process_input(event, &chart).expect("Should trigger judgment");
        assert_eq!(result.judgment, Judgment::Bad);
        assert_eq!(result.note_index, 0);
    }

    #[test]
    fn test_ignored_input_too_early() {
        let chart = create_test_chart();
        let mut judge = JudgeMachine::new();

        // Note 0 at 1.0.
        // Bad window 0.200. Capture window.
        // Hit at 0.5 -> -0.5 -> Too early.
        let event = InputEvent {
            timestamp: 0.5,
            button: Button::Key1,
            pressed: true,
        };

        let result = judge.process_input(event, &chart);
        assert!(result.is_none());

        // Ensure index did not advance
        assert_eq!(judge.next_note_index[1], 0);
    }

    #[test]
    fn test_check_misses() {
        let chart = create_test_chart();
        let mut judge = JudgeMachine::new();

        // Current time 1.0. Note 0 is at 1.0. Not missed.
        let misses = judge.check_misses(1.0, &chart);
        assert!(misses.is_empty());

        // Current time 1.15. Note 0 (1.0) + Bad (0.2) = 1.2. Not missed.
        let misses = judge.check_misses(1.15, &chart);
        assert!(misses.is_empty());

        // Current time 1.21. Note 0 missed.
        let misses = judge.check_misses(1.21, &chart);
        assert_eq!(misses.len(), 1);
        assert_eq!(misses[0].judgment, Judgment::Poor);
        assert_eq!(misses[0].note_index, 0);

        // Next time we check, Note 0 should not be reported again.
        // Note 1 (Lane 2, Time 1.5) is next.
        // Note 2 (Lane 1, Time 2.0) is next for Lane 1.

        let misses = judge.check_misses(1.22, &chart);
        assert!(misses.is_empty());

        // Wait until Note 2 is missed (Time 2.0 + 0.2 = 2.2).
        let misses = judge.check_misses(2.21, &chart);
        assert_eq!(misses.len(), 2);

        // Verify we got both.
        let found_note_1 = misses.iter().any(|r| r.note_index == 1);
        let found_note_2 = misses.iter().any(|r| r.note_index == 2);

        assert!(found_note_1);
        assert!(found_note_2);
    }

    #[test]
    fn test_lane_independence_out_of_order_input() {
        let chart = create_test_chart();
        let mut judge = JudgeMachine::new();

        // Note 0 (Lane 1, 1.0).
        // Note 1 (Lane 2, 1.5).

        // Hit Lane 2 first (at 1.5).
        let event = InputEvent {
            timestamp: 1.5,
            button: Button::Key2,
            pressed: true,
        };
        let result = judge.process_input(event, &chart).expect("Hit Lane 2");
        assert_eq!(result.note_index, 1);

        // Lane 1 should still be available at 1.0
        let event = InputEvent {
            timestamp: 1.0,
            button: Button::Key1,
            pressed: true,
        };
        let result = judge.process_input(event, &chart).expect("Hit Lane 1");
        assert_eq!(result.note_index, 0);

        // Verify no misses generated if we check misses at 1.6
        let misses = judge.check_misses(1.6, &chart);
        assert!(misses.is_empty());
    }
}
