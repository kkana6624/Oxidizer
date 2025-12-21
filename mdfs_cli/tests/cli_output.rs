use std::{
    env,
    fs,
    process::Command,
};

fn norm_newlines(s: &str) -> String {
    s.replace("\r\n", "\n").replace('\r', "")
}

#[test]
fn compile_error_output_format_is_stable() {
    let exe = env!("CARGO_BIN_EXE_mdfs_cli");

    let tmp = env::temp_dir().join(format!(
        "oxidizer_mdfs_cli_compile_error_format_{}.mdfs",
        std::process::id()
    ));

    fs::write(
        &tmp,
        "@title T\n@artist A\n@version 2.2\ntrack: |\n  @bpm 120\n  @div 4\n  ..X.....\n",
    )
    .unwrap();

    let output = Command::new(exe)
        .args(["compile", tmp.to_str().unwrap()])
        .output()
        .unwrap();

    assert!(!output.status.success());
    assert_eq!(output.status.code(), Some(1));

    let stderr = norm_newlines(&String::from_utf8_lossy(&output.stderr));
    assert!(stderr.contains("Error: compile failed: "));
    assert!(stderr.contains("Caused by:"));
    assert!(stderr.contains(
        "E4001: undefined step char (lane=2, char='X', context=..X.....) (line 7)"
    ));
}

#[test]
fn compile_missing_input_file_is_e2001() {
    let exe = env!("CARGO_BIN_EXE_mdfs_cli");

    let missing = env::temp_dir().join(format!(
        "oxidizer_mdfs_cli_missing_input_{}.mdfs",
        std::process::id()
    ));
    let _ = fs::remove_file(&missing);

    let output = Command::new(exe)
        .args(["compile", missing.to_str().unwrap()])
        .output()
        .unwrap();

    assert!(!output.status.success());
    assert_eq!(output.status.code(), Some(1));

    let stderr = norm_newlines(&String::from_utf8_lossy(&output.stderr));
    assert!(stderr.contains("Error: compile failed: "));
    // OS により末尾の I/O error 文字列は変わるため、prefix と line だけ固定
    assert!(stderr.contains("E2001: failed to read input .mdfs:"));
    assert!(stderr.contains("(line 0)"));
}

#[test]
fn compile_success_writes_output_json() {
    let exe = env!("CARGO_BIN_EXE_mdfs_cli");

    let dir = env::temp_dir().join(format!(
        "oxidizer_mdfs_cli_compile_success_{}",
        std::process::id()
    ));
    fs::create_dir_all(&dir).unwrap();

    let input = dir.join("in.mdfs");
    let output_path = dir.join("out.mdf.json");
    fs::write(
        &input,
        "@title T\n@artist A\n@version 2.2\ntrack: |\n  @bpm 120\n  @div 4\n  ..N.....\n",
    )
    .unwrap();

    let out = Command::new(exe)
        .args([
            "compile",
            input.to_str().unwrap(),
            "-o",
            output_path.to_str().unwrap(),
        ])
        .output()
        .unwrap();

    assert!(out.status.success());
    assert!(output_path.exists());

    let json = fs::read_to_string(&output_path).unwrap();
    let v: serde_json::Value = serde_json::from_str(&json).unwrap();
    assert!(v.get("meta").is_some());
    assert!(v.get("notes").is_some());
}

#[test]
fn help_mentions_compile_subcommand() {
    let exe = env!("CARGO_BIN_EXE_mdfs_cli");

    let output = Command::new(exe).arg("--help").output().unwrap();

    assert!(output.status.success());
    let stdout = norm_newlines(&String::from_utf8_lossy(&output.stdout));

    // clap のヘルプ文言は細部が変わり得るため、存在確認だけに留める。
    assert!(stdout.contains("compile"));
}

#[test]
fn compile_output_write_failure_is_reported_stably() {
    let exe = env!("CARGO_BIN_EXE_mdfs_cli");

    let dir = env::temp_dir().join(format!(
        "oxidizer_mdfs_cli_compile_write_failure_{}",
        std::process::id()
    ));
    fs::create_dir_all(&dir).unwrap();

    let input = dir.join("in.mdfs");
    fs::write(
        &input,
        "@title T\n@artist A\n@version 2.2\ntrack: |\n  @bpm 120\n  @div 4\n  ..N.....\n",
    )
    .unwrap();

    // 親ディレクトリが存在しないパスへ書き込ませて、OS 依存の I/O error 本文は固定せず
    // `failed to write:` の prefix だけを検証する。
    let missing_parent = dir.join(format!("missing_dir_{}", std::process::id()));
    let _ = fs::remove_dir_all(&missing_parent);
    let output_path = missing_parent.join("out.mdf.json");

    let out = Command::new(exe)
        .args([
            "compile",
            input.to_str().unwrap(),
            "-o",
            output_path.to_str().unwrap(),
        ])
        .output()
        .unwrap();

    assert!(!out.status.success());
    assert_eq!(out.status.code(), Some(1));

    let stderr = norm_newlines(&String::from_utf8_lossy(&out.stderr));
    assert!(stderr.contains("Error: failed to write:"));
    assert!(stderr.contains("failed to write:"));
    assert!(stderr.contains("out.mdf.json"));
    assert!(stderr.contains("Caused by:"));
}
