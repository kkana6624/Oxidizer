use super::*;
use crate::{
    generate::pass2_generate,
    parser::{RevSpec, SoundSpec, TrackLine},
};
use mdf_schema::{Microseconds, NoteKind};
use std::{
    collections::HashMap,
    fs,
    path::PathBuf,
    time::{SystemTime, UNIX_EPOCH},
};

#[test]
fn compile_minimal_tap_without_manifest_if_no_sound_ids() {
    let src = r#"
@title T
@artist A
@version 2.2
track: |
  @bpm 120
  @div 4
  ........
  ..N.....
"#;

    let chart = compile_str(src).unwrap();
    assert_eq!(chart.meta.title, "T");
    assert_eq!(chart.notes.len(), 1);
    assert_eq!(chart.notes[0].col, 2);
    assert_eq!(chart.notes[0].sound_id, None);
    assert!(chart.meta.total_duration_us > 0);
}

#[test]
fn mss_generates_reverse_checkpoints_from_markers_and_rev_at() {
    let src = r#"
@title T
@artist A
@version 2.2
track: |
  @bpm 120
  @div 4
  m....... : [] @rev_at 2,3
  !.......
  ........
  m.......
"#;

    let chart = compile_str(src).unwrap();
    assert_eq!(chart.notes.len(), 1);
    let n = &chart.notes[0];
    match &n.kind {
        NoteKind::MultiSpinScratch {
            end_time_us,
            reverse_checkpoints_us,
        } => {
            assert!(*end_time_us > n.time_us);
            assert!(!reverse_checkpoints_us.is_empty());
        }
        _ => panic!("unexpected kind"),
    }
}

#[test]
fn compile_with_manifest_loads_resources_and_validates_sound_ids() {
    let tmp_base = std::env::temp_dir().join(format!(
        "oxidizer_mdfs_compiler_test_{}_{}",
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    fs::create_dir_all(&tmp_base).unwrap();
    let manifest_path = tmp_base.join("sounds.json");
    fs::write(
        &manifest_path,
        r#"{
    "K01": "kick.wav",
    "SE_END": "end.wav"
}"#,
    )
    .unwrap();

    let src = r#"
@title T
@artist A
@version 2.2
@sound_manifest sounds.json
track: |
  @bpm 120
  @div 4
  ..N..... : K01
  ........ : SE_END
"#;

    let chart = compile_str_with_options(
        src,
        CompileOptions {
            base_dir: Some(tmp_base.clone()),
        },
    )
    .unwrap();

    assert_eq!(chart.resources.get("K01").unwrap(), "kick.wav");
    assert_eq!(chart.notes.len(), 1);
    assert_eq!(chart.notes[0].sound_id.as_deref(), Some("K01"));
    assert_eq!(chart.bgm_events.len(), 1);
    assert_eq!(chart.bgm_events[0].sound_id, "SE_END");
}

#[test]
fn repo_example_compiles() {
    let crate_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let example = crate_dir.join("..").join("examples").join("minimal.mdfs");
    let chart = compile_file(&example).unwrap();
    assert_eq!(chart.meta.title, "Minimal Example");
    assert!(!chart.notes.is_empty());
}

#[test]
fn error_code_unknown_directive_is_e1006() {
    let src = r#"
@title T
@artist A
@version 2.2
track: |
    @bpm 120
    @div 4
    @unknown 1
    ..N.....
"#;
    let err = compile_str(src).unwrap_err();
    assert_eq!(err.code, "E1006");
    assert_eq!(err.kind, CompileErrorKind::Parse);
}

#[test]
fn error_code_short_step_line_is_e1101() {
    let src = r#"
@title T
@artist A
@version 2.2
track: |
    @bpm 120
    @div 4
    ...
"#;
    let err = compile_str(src).unwrap_err();
    assert_eq!(err.code, "E1101");
    assert_eq!(err.kind, CompileErrorKind::Parse);
}

#[test]
fn error_code_scratch_only_on_non_scratch_is_e4002() {
    let src = r#"
@title T
@artist A
@version 2.2
track: |
    @bpm 120
    @div 4
    .S......
"#;
    let err = compile_str(src).unwrap_err();
    assert_eq!(err.code, "E4002");
    assert_eq!(err.kind, CompileErrorKind::Validation);
}

#[test]
fn error_code_missing_track_is_e1101() {
    let src = r#"
@title T
@artist A
@version 2.2
"#;
    let err = compile_str(src).unwrap_err();
    assert_eq!(err.code, "E1101");
    assert_eq!(err.kind, CompileErrorKind::Parse);
}

#[test]
fn error_code_sound_id_without_manifest_is_e2101_with_line() {
    let src = r#"
@title T
@artist A
@version 2.2
track: |
  @bpm 120
  @div 4
  ..N..... : K01
"#;
    let err = compile_str(src).unwrap_err();
    assert_eq!(err.code, "E2101");
    assert_eq!(err.kind, CompileErrorKind::Semantic);
    assert_eq!(err.line, 8);
    assert_eq!(err.lane, Some(2));
    assert!(err.message.contains("sound_id=K01"));
    assert!(err.message.contains("lane=2"));
}

#[test]
fn error_code_sound_id_missing_in_manifest_is_e2101_with_sound_id_and_lane() {
    let tmp_base = std::env::temp_dir().join(format!(
        "oxidizer_mdfs_compiler_test_manifest_missing_id_{}_{}",
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    fs::create_dir_all(&tmp_base).unwrap();
    fs::write(tmp_base.join("sounds.json"), r#"{"OTHER":"x.wav"}"#).unwrap();

    let src = "@title T\n@artist A\n@version 2.2\n@sound_manifest sounds.json\ntrack: |\n  @bpm 120\n  @div 4\n  ..N..... : K01\n";

    let err = compile_str_with_options(
        src,
        CompileOptions {
            base_dir: Some(tmp_base.clone()),
        },
    )
    .unwrap_err();

    assert_eq!(err.code, "E2101");
    assert_eq!(err.kind, CompileErrorKind::Semantic);
    assert_eq!(err.line, 8);
    assert_eq!(err.lane, Some(2));
    assert!(err.message.contains("sound_id=K01"));
    assert!(err.message.contains("lane=2"));
}

#[test]
fn error_code_step_duration_rounded_to_zero_is_e3005() {
    let src = r#"
@title T
@artist A
@version 2.2
track: |
  @bpm 1000000000000
  @div 4
  ..N.....
"#;
    let err = compile_str(src).unwrap_err();
    assert_eq!(err.code, "E3005");
    assert_eq!(err.kind, CompileErrorKind::TimeMap);
    assert_eq!(err.line, 8);
}

#[test]
fn error_code_e4004_tap_then_hold_start_same_time_lane() {
    let mut cells1 = ['.'; 8];
    cells1[1] = 'N';
    let mut cells2 = ['.'; 8];
    cells2[1] = 'l';

    let track = vec![
        TrackLine::Step {
            line: 1,
            cells: cells1,
            sound: SoundSpec::None,
            rev: RevSpec::default(),
        },
        TrackLine::Step {
            line: 2,
            cells: cells2,
            sound: SoundSpec::None,
            rev: RevSpec::default(),
        },
    ];

    let step_times: Vec<Microseconds> = vec![0, 0];
    let resources = HashMap::<String, String>::new();

    let err = pass2_generate(&track, &step_times, &resources).unwrap_err();
    assert_eq!(err.code, "E4004");
    assert_eq!(err.kind, CompileErrorKind::Validation);
    assert_eq!(err.step_index, Some(1));
    assert_eq!(err.time_us, Some(0));
    assert_eq!(err.lane, Some(1));
    assert!(err.message.contains("lane=1"));
    assert!(err.message.contains("time_us=0"));
    assert!(err.message.contains("overlaps"));
}

#[test]
fn error_code_e4004_hold_start_then_tap_same_time_lane() {
    let mut cells1 = ['.'; 8];
    cells1[1] = 'l';
    let mut cells2 = ['.'; 8];
    cells2[1] = 'N';

    let track = vec![
        TrackLine::Step {
            line: 1,
            cells: cells1,
            sound: SoundSpec::None,
            rev: RevSpec::default(),
        },
        TrackLine::Step {
            line: 2,
            cells: cells2,
            sound: SoundSpec::None,
            rev: RevSpec::default(),
        },
    ];

    let step_times: Vec<Microseconds> = vec![0, 0];
    let resources = HashMap::<String, String>::new();

    let err = pass2_generate(&track, &step_times, &resources).unwrap_err();
    assert_eq!(err.code, "E4004");
    assert!(err.message.contains("lane=1"));
    assert!(err.message.contains("time_us=0"));
    assert!(err.message.contains("overlaps"));
}

#[test]
fn error_code_missing_bpm_before_steps_is_e3001() {
    let src = "@title T\n@artist A\n@version 2.2\ntrack: |\n  @div 4\n  ..N.....\n";
    let err = compile_str(src).unwrap_err();
    assert_eq!(err.code, "E3001");
    assert_eq!(err.kind, CompileErrorKind::TimeMap);
    assert_eq!(err.line, 6);
}

#[test]
fn error_code_missing_div_before_steps_is_e3002() {
    let src = "@title T\n@artist A\n@version 2.2\ntrack: |\n  @bpm 120\n  ..N.....\n";
    let err = compile_str(src).unwrap_err();
    assert_eq!(err.code, "E3002");
    assert_eq!(err.kind, CompileErrorKind::TimeMap);
    assert_eq!(err.line, 6);
}

#[test]
fn error_code_invalid_manifest_json_is_e2002() {
    let tmp_base = std::env::temp_dir().join(format!(
        "oxidizer_mdfs_compiler_test_manifest_invalid_json_{}_{}",
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    fs::create_dir_all(&tmp_base).unwrap();
    fs::write(tmp_base.join("sounds.json"), "not json").unwrap();

    let src = "@title T\n@artist A\n@version 2.2\n@sound_manifest sounds.json\ntrack: |\n  @bpm 120\n  @div 4\n  ........\n";
    let err = compile_str_with_options(
        src,
        CompileOptions {
            base_dir: Some(tmp_base.clone()),
        },
    )
    .unwrap_err();
    assert_eq!(err.code, "E2002");
    assert_eq!(err.kind, CompileErrorKind::IO);
    assert_eq!(err.line, 4);
}

#[test]
fn error_code_invalid_manifest_values_is_e2003() {
    let tmp_base = std::env::temp_dir().join(format!(
        "oxidizer_mdfs_compiler_test_manifest_invalid_values_{}_{}",
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    fs::create_dir_all(&tmp_base).unwrap();
    fs::write(tmp_base.join("sounds.json"), r#"{"K01":""}"#).unwrap();

    let src = "@title T\n@artist A\n@version 2.2\n@sound_manifest sounds.json\ntrack: |\n  @bpm 120\n  @div 4\n  ........\n";
    let err = compile_str_with_options(
        src,
        CompileOptions {
            base_dir: Some(tmp_base.clone()),
        },
    )
    .unwrap_err();
    assert_eq!(err.code, "E2003");
    assert_eq!(err.kind, CompileErrorKind::IO);
    assert_eq!(err.line, 4);
}

#[test]
fn error_code_multiple_sound_manifest_is_e2004() {
    let src = "@title T\n@artist A\n@version 2.2\n@sound_manifest a.json\n@sound_manifest b.json\ntrack: |\n  @bpm 120\n  @div 4\n  ........\n";
    let err = compile_str(src).unwrap_err();
    assert_eq!(err.code, "E2004");
    assert_eq!(err.kind, CompileErrorKind::IO);
    assert_eq!(err.line, 5);
}

#[test]
fn error_code_rev_directive_outside_mss_hmss_is_e4201() {
    let src = "@title T\n@artist A\n@version 2.2\ntrack: |\n  @bpm 120\n  @div 4\n  ..N..... @rev_at 2\n";
    let err = compile_str(src).unwrap_err();
    assert_eq!(err.code, "E4201");
    assert_eq!(err.kind, CompileErrorKind::Semantic);
    assert_eq!(err.line, 7);
}

#[test]
fn error_code_unclosed_toggle_is_e4101() {
    let src = "@title T\n@artist A\n@version 2.2\ntrack: |\n  @bpm 120\n  @div 4\n  .l......\n";
    let err = compile_str(src).unwrap_err();
    assert_eq!(err.code, "E4101");
    assert_eq!(err.kind, CompileErrorKind::Validation);
    assert_eq!(err.line, 7);
    assert_eq!(err.step_index, Some(0));
    assert_eq!(err.time_us, Some(0));
    assert_eq!(err.lane, Some(1));
    assert!(err.message.contains("lane=1"));
    assert!(err.message.contains("start_line=7"));
    assert!(err.message.contains("start_time_us="));
}

#[test]
fn error_code_marker_during_bss_is_e4102() {
    let src = "@title T\n@artist A\n@version 2.2\ntrack: |\n  @bpm 120\n  @div 4\n  b.......\n  !.......\n";
    let err = compile_str(src).unwrap_err();
    assert_eq!(err.code, "E4102");
    assert_eq!(err.kind, CompileErrorKind::Validation);
    assert_eq!(err.line, 8);
}

#[test]
fn parse_error_e1101_includes_context() {
    let src = "@title T\n@artist A\n@version 2.2\ntrack: |\n  @bpm 120\n  @div 4\n  ...\n";
    let err = compile_str(src).unwrap_err();
    assert_eq!(err.code, "E1101");
    assert!(err.message.contains("context="));
    assert_eq!(err.context.as_deref(), Some("..."));
}

#[test]
fn parse_error_e1001_invalid_sound_spec_token_includes_context() {
    let src = "@title T\n@artist A\n@version 2.2\ntrack: |\n  @bpm 120\n  @div 4\n  ..N..... : K01 K02\n";
    let err = compile_str(src).unwrap_err();
    assert_eq!(err.code, "E1001");
    assert!(err.message.contains("context="));
    assert_eq!(err.context.as_deref(), Some("..N..... : K01 K02"));
}

#[test]
fn parse_error_e1002_sound_spec_wrong_slots_includes_context() {
    let src = "@title T\n@artist A\n@version 2.2\ntrack: |\n  @bpm 120\n  @div 4\n  ..N..... : [K01,-,-,-,-,-,-]\n";
    let err = compile_str(src).unwrap_err();
    assert_eq!(err.code, "E1002");
    assert!(err.message.contains("context="));
    assert_eq!(err.context.as_deref(), Some("..N..... : [K01,-,-,-,-,-,-]"));
}

#[test]
fn parse_error_e1003_sound_spec_empty_slot_includes_context() {
    let src = "@title T\n@artist A\n@version 2.2\ntrack: |\n  @bpm 120\n  @div 4\n  ..N..... : [K01,,-,-,-,-,-,-]\n";
    let err = compile_str(src).unwrap_err();
    assert_eq!(err.code, "E1003");
    assert!(err.message.contains("context="));
    assert!(err.message.contains("lane=1"));
    assert_eq!(err.lane, Some(1));
    assert_eq!(err.context.as_deref(), Some("..N..... : [K01,,-,-,-,-,-,-]"));
}

#[test]
fn parse_error_e4001_undefined_step_char_includes_lane_char_context() {
    let src = "@title T\n@artist A\n@version 2.2\ntrack: |\n  @bpm 120\n  @div 4\n  ..X.....\n";
    let err = compile_str(src).unwrap_err();
    assert_eq!(err.code, "E4001");
    assert_eq!(err.line, 7);
    assert_eq!(err.lane, Some(2));
    assert_eq!(err.context.as_deref(), Some("..X....."));
    assert!(err.message.contains("lane=2"));
    assert!(err.message.contains("char='X'"));
    assert!(err.message.contains("context=..X....."));
}

#[test]
fn parse_error_e4001_char_not_allowed_on_scratch_lane_includes_context() {
    let src = "@title T\n@artist A\n@version 2.2\ntrack: |\n  @bpm 120\n  @div 4\n  l.......\n";
    let err = compile_str(src).unwrap_err();
    assert_eq!(err.code, "E4001");
    assert_eq!(err.line, 7);
    assert_eq!(err.lane, Some(0));
    assert_eq!(err.context.as_deref(), Some("l......."));
    assert!(err.message.contains("lane=0"));
    assert!(err.message.contains("char='l'"));
    assert!(err.message.contains("context=l......."));
}

#[test]
fn parse_error_e4002_scratch_only_char_on_non_scratch_includes_lane_char_context() {
    let src = "@title T\n@artist A\n@version 2.2\ntrack: |\n  @bpm 120\n  @div 4\n  .S......\n";
    let err = compile_str(src).unwrap_err();
    assert_eq!(err.code, "E4002");
    assert_eq!(err.line, 7);
    assert_eq!(err.lane, Some(1));
    assert_eq!(err.context.as_deref(), Some(".S......"));
    assert!(err.message.contains("lane=1"));
    assert!(err.message.contains("char='S'"));
    assert!(err.message.contains("context=.S......"));
}

#[test]
fn parse_error_e4003_bang_on_non_scratch_includes_lane_context() {
    let src = "@title T\n@artist A\n@version 2.2\ntrack: |\n  @bpm 120\n  @div 4\n  .!......\n";
    let err = compile_str(src).unwrap_err();
    assert_eq!(err.code, "E4003");
    assert_eq!(err.line, 7);
    assert_eq!(err.lane, Some(1));
    assert_eq!(err.context.as_deref(), Some(".!......"));
    assert!(err.message.contains("lane=1"));
    assert!(err.message.contains("context=.!......"));
}
