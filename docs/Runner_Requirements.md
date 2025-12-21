# Runner（mdf_runner）要件定義（MVP → 拡張）

本書は Rust workspace「Oxidizer」における Runner（`mdf_runner`）の要件定義である。ここでの Runner は `.mdf`（= `mdf_schema::MdfChart`）を入力として、**絶対時間（us）**を基準に「いつ・何を判定/通知するか」を決定する。

- 対象: `mdf_runner` の仕様・要件・インタフェース・テスト方針
- 非対象: 具体実装、UI/音声エンジンの実装（通知までを責務とする）

## 0. 前提（不変の制約）

- 判定の基準は **絶対時間（`Microseconds = u64`）**。
- `.mdf` はコンパイル済みの「時間順イベント列」であり、Runner は小節/拍などの音楽的概念を根拠にしない。
- `visual_events` / `speed_events` は表示補助であり、**判定根拠に利用しない**（空でも動作できる）。

## 1. データ境界（Compiler と Runner の責務分離）

### 1.1 Compiler（`mdfs_compiler`）の責務

- `.mdfs` の解釈（ディレクティブ、丸め規則、Toggleロジックなど）
- `MdfChart` 生成（ノーツ/イベントの **絶対時刻** 列生成まで）
- MSS/HMSS の `reverse_checkpoints_us` の生成（`@rev_every` / `@rev_at` などから）

> 重要: 判定窓（許容誤差）や入力解釈（キーON/OFF、スクラッチ回転の意味付け）は Runner 側の責務とする。

### 1.2 Runner（`mdf_runner`）の責務

- `.mdf` を読み込み、時刻順に進行させながら判定・通知する
- 判定窓（許容誤差）と評価順序（同時刻イベント含む）を定義し、決定論的に処理する
- Hold系（CN/HCN/BSS/HBSS/MSS/HMSS）の状態遷移（復帰ルール含む）を保持する
- BGM/SE/表示に必要な情報を **イベントとして通知** する（再生/描画は外部に委譲可能）

## 2. Runner の利用シナリオ（3分類）

### 2.1 オフライン検証（入力ログ → 判定結果）

目的: 入力ログ（タイムスタンプ付き）を与え、判定結果と過程を再現可能に得る。

- 入力
  - `MdfChart`
  - `TimedInputEvent` の列（`timestamp_us` 必須）
- 出力
  - ノート/チェックポイントの確定結果（OK/MISS、必要なら時差）
  - 途中経過（Broken/Recovered/Checkpoint達成など）
  - BGM通知のログ（`sound_id`, `at_us`）
- 要件
  - 完全決定論（同じ入力→同じ出力）
  - 実時間に依存しない（sleep不要）
  - 改行・OS差分に依存しない出力が可能な設計（テスト方針に合わせる）

### 2.2 リアルタイム再生（時間基準で進める）

目的: 現在時刻 `now_us` を進めつつ、入力を取り込み、判定/通知をフレーム単位で生成する。

- 入力
  - `now_us` の更新
  - フレーム内に到来した `TimedInputEvent`（もしくはそのバッチ）
- 出力
  - そのフレームで確定した `RunnerEvent` 群
  - そのフレームで鳴らすべき BGM/SE 通知
  - 可視化/デバッグ用の「要求状態スナップショット」（任意）
- 要件
  - `now_us` が大きく飛んでも処理できる（早送り）
  - 入力イベントが遅延しても `timestamp_us` を尊重できる設計が望ましい

### 2.3 可視化/デバッグ（要求/未達イベントの表示）

目的: 現在時刻に対して「次に何が要求されているか」「何が未達でどの状態か」を表示/取得する。

- 入力
  - `now_us`
- 出力
  - 近傍の対象ノーツ一覧（次Tap、保持中Hold、次checkpoint、次BGM等）
  - 未達理由の説明（例: 「checkpoint#2 の逆回転未達」）
- 要件
  - 判定ロジックと同一のソースから算出し、表示と判定が食い違わない

### 2.4 MVPのI/O方針（現時点の決定）

- Runner単体としては **CLIへのログ出力のみ** でよい（UI/音声エンジンはこの段階では不要）
- ただしテストはログ文字列に過度に依存させず、OS差・改行差に強い方式を採用する（後述）

## 3. 入出力インタフェース案（MVP）

### 3.1 入力（案）

- チャート
  - `&MdfChart`
- 時刻
  - `now_us: Microseconds`
- 入力イベント（必須）
  - `TimedInputEvent { timestamp_us, event }`
  - キー
    - `KeyEvent { col: u8, edge: Press | Release }`
  - スクラッチ（MVPは抽象化してよい）
    - 案A（簡単）: `ScratchEvent { dir: Cw | Ccw, kind: Move | Stop }`
    - 案B（現実寄り）: `ScratchEvent { delta: i32 }`（Runnerでdir/停止を推定）

> 推奨: オフライン検証・遅延入力に強くするため `timestamp_us` を必須化する。

#### スクラッチ入力抽象の指針（ロバスト性）

プログラミング的なロバスト性（デバイス差・取りこぼし耐性・将来拡張）を優先するなら、Runnerの外で「方向イベント」に確定して渡すより、**より生の入力（`delta`）を受け取り、Runner側で意味付けを行える設計**が強い。

- 利点（`delta`）
  - デバイス/ドライバごとのイベント粒度差を吸収しやすい（閾値・ヒステリシスで調整可能）
  - “逆回転” と “停止” の判定を同じ時系列上で一貫して扱える
  - 将来的なDP/別デバイスでも入力層を差し替えやすい
- 利点（`dir`イベント）
  - 実装・テストが単純（Runner内推定ロジックが不要）
  - 検証用途で「逆回転入力があった/ない」を明確に表現できる

本プロジェクトは「入力の正確性計測」を最優先とするため、**`delta` を受け取る方針**とする。

#### delta感度（Sensitivity）とキャリブレーション

`delta` を採用する場合、デバイスごとの分解能差（1イベントあたりの増分や発火頻度）により「少し触れただけで反応する / 回しても反応しない」が起こりうる。
このため Runner 側で感度を調整可能にしておくことを要件とする。

- `RunnerConfig` に含めることを推奨するパラメータ例
  - `scratch_min_abs_delta`: 入力1回を有効な回転として扱う最小絶対delta（ノイズ除去）
  - `scratch_dir_threshold`: 方向（CW/CCW）を確定するためのしきい値（ヒステリシスを持たせてもよい）
  - `scratch_move_accumulation_us`: 「Move」と見なすためのdelta蓄積の評価時間幅（瞬時判定にするなら0でもよい）
  - `scratch_ticks_per_rotation`: 1回転相当とみなすdelta総量（統計/可視化や閾値設計に利用）
  - `scratch_deadzone_abs_delta`: 近傍ノイズを無視するデッドゾーン（`scratch_min_abs_delta` と統合可）

将来的にはキャリブレーション（実測に基づく自動推定）機能が有用になり得るが、MVPでは「設定で吸収できる」ことを優先する。

### 3.2 出力（案）

- 確定したイベント列 `Vec<RunnerEvent>`（例）
  - `NoteJudged { note_id, result, at_us, timing_error_us? }`
  - `HoldStateChanged { note_id, state, at_us }`（Broken/Recovered等）
  - `CheckpointJudged { note_id, checkpoint_index, result, at_us }`
  - `BgmTrigger { sound_id, at_us }`
- 任意: スナップショット `RunnerSnapshot`
  - 次に要求される操作（押す/保持/離す/回す/逆回転）
  - 保持系の内部状態（次の締切、次checkpoint 等）

#### timing_error_us の定義（共通）

- `timing_error_us` は負値を取りうるため、型は `i64` 相当を想定する（内部表現は実装設計に委ねるが「符号あり」であることを仕様とする）
- `timing_error_us = satisfied_time_us - target_time_us`
- `target_time_us` はノート/要求点の基準時刻（Tapなら `Note.time_us`、CN終点なら `end_time_us`、MSS checkpointなら `checkpoint_us`）
- `satisfied_time_us` は「判定窓内で要件が満たされた最初の時刻」とする
  - ただし、開始時点ON許容・ウィンドウ内OFF許容のように“状態”で満たす要件については、**`target_time_us` の時点で既に要件状態を満たしている場合**は `satisfied_time_us = target_time_us` としてよい
    - 例: CN開始で `target_time_us = t_s` の瞬間にキーがONなら `timing_error_us = 0`

符号の意味:

- `timing_error_us < 0`: Early（FAST側）
- `timing_error_us > 0`: Late（SLOW側）
- `timing_error_us == 0`: ちょうど（F-GREATの条件にも使う）

### 3.3 note_id の扱い（重要）

`mdf_schema` にはノートIDが存在しないため、Runner 内部で安定した識別子が必要。

- MVP案: Runner初期化時に `notes` をソートし、連番 `note_id` を割り当てる
- 代替案: 元配列インデックスを `note_id` にする（ただし入力/表示側が `.mdf` の並びに依存しやすい）

安定性の補足:

- 将来的な拡張やデータ不整合で「同時刻・同レーン・同種別」のノートが存在しても `note_id` が揺れないよう、**完全な順序付け**を行う
  - 推奨: ソートキーの末尾に「元配列インデックス」を含めてタイブレークする（例: `(time_us, col, kind, original_index)`）
  - RustのソートAPIが安定かどうかに依存しないよう、キー設計で完全順序にしておく

## 4. 内部モデル（最小構成）

### 4.1 スケジューリング（deadline で確定）

- `notes` と `bgm_events` を `time_us` で処理する
- ただし判定には窓があるため、「時刻到達」ではなく **締切 `deadline_us` を過ぎたらMISS確定** できるようにする
  - Tap: `deadline = t + W_tap_late`
  - CN開始: `deadline = t_start + W_cn_start_late`
  - CN終点（離し）: `deadline = t_end + W_cn_end_late`
  - MSS checkpoint: `deadline = t_cp + W_rev_late`

#### 入力処理の優先順位（重要）

トレーニング用途として「ノートを正確に入力できたか」の計測を最優先するため、同一時刻内での処理順は次を原則とする。

1. そのフレーム/ステップで到来した **入力イベントを先に適用**（`timestamp_us` 順、同時刻は到着順でよいが仕様として固定する）
2. 次に、判定窓内で達成可能になった項目を確定（OK側）
3. 最後に、`deadline_us` 超過による失敗確定（MISS側）

これにより「入力が間に合っているのに deadline 処理順でMISSになる」ことを防ぐ。

### 4.2 Hold系の状態遷移（共通骨格）

- 共通状態（例）
  - `NotStarted` → `Active` → `Ended`
  - `Active` 中に `Broken` を挟む（ヘル系は `Recovered` を経由）

- 非ヘル（CN/BSS/MSS）
  - `Broken` が発生した瞬間に **そのノートは最終的に失敗確定**（復帰なし）

- ヘル（HCN/HBSS/HMSS）
  - `Broken` になっても追跡継続し、再入力で復帰可能
  - 復帰は「復帰時刻以降の要求」にのみ有効（遡及しない）
  - HMSSは「切れている間に到来した checkpoint は保持しない」

### 4.3 reverse_checkpoints_us と終点逆回転

- MSS/HMSS
  - `reverse_checkpoints_us[]` と `end_time_us` を「逆回転要求点」として扱う
  - 各要求点 `t` に対し、窓 `[t - W_rev_early, t + W_rev_late]` 内に **逆回転入力** を1回以上観測したら達成
- BSS/HBSS
  - 逆回転要求点は `end_time_us` のみ

- 同時刻/近接の扱い
  - checkpointが近接または同一時刻の場合、1回の逆回転入力で複数達成を許すかは仕様決めが必要（決定事項へ）

### 4.4 スクラッチ「回転継続/停止」の判定

- 仕様上「回転停止」はコンボ切れの要因になりうるため、Runner側に `spin_gap_us` のような閾値が必要
- 例: 「最後の回転入力から `spin_gap_us` 以上無入力なら停止扱い」

## 5. 追加要件: IIDX標準準拠 CN/HCN（Runner仕様として文章化）

本節は外部資料の引用ではなく、Runnerとして採用する「標準ルール」を明文化する。

### 5.1 用語

- `t_s`: CN/HCN の開始時刻（`Note.time_us`）
- `t_e`: CN/HCN の終点時刻（`end_time_us`）
- 入力は `Press`（キーONエッジ）と `Release`（キーOFFエッジ）で表現する

### 5.2 CN（Charge Note）採用ルール

#### CN開始（開始判定）

- 要求: 開始窓 `[t_s - W_start_early, t_s + W_start_late]` のどこかで、対象キーが **ON状態** であること（開始時点ONを許容）
  - 具体化（入力イベントがエッジで来る前提）:
    - 窓開始時点のキー状態を既知にし（初期状態はOFF）、窓内の `Press/Release` を反映した状態遷移から「ON状態になった瞬間」を検出する
    - すでにONで窓に入った場合は、その時点で開始要件を満たす
- 成功: start確定後 `Active` に遷移
- 失敗: `t_s + W_start_late` を過ぎても窓内でON状態にならなければ `MISS` 確定（以後追跡しない）

補足（押しっぱなし接続）:

- 直前のTapで押しっぱなしのまま開始窓に入った場合でも、上記条件を満たすならCN開始として成立しうる（接続を許容）
- ただし、より厳密に「1回のON（押下）を複数ノートに消費させない」モデル（入力消費モデル）を採用する余地はある
  - 例: 1つの `Press` を Tap/CN開始のいずれか1回だけに割り当てる
  - 本書ではMVPとして実装複雑度を上げないため、まずは状態ベースの要件を採用する（必要になれば将来拡張で導入）

#### CN保持（保持要件）

- 要求: `Active` 中、`t < t_e` に `Release` が発生してはいけない
- 失敗: `Release` が発生した瞬間に `MISS` 確定（復帰なし）

#### CN終点（離し判定）

- 要求: 終点窓 `[t_e - W_end_early, t_e + W_end_late]` のどこかで、対象キーが **OFF状態** であること（ウィンドウ内OFFを許容）
  - 具体化:
    - 窓内で `Release` が発生してOFFになった場合は、その時点で要件を満たす
    - すでにOFFで窓に入った場合は、その時点で要件を満たす
- 成功: 上記の条件を満たした時点で完了（OK）
- 失敗: `t_e + W_end_late` を過ぎても窓内でOFF状態にならなければ `MISS` 確定

### 5.3 HCN（Hell Charge Note）採用ルール

#### HCN開始

- CNと同一（開始時点ONを許容し、開始窓内でON状態にならなければ締切で `MISS`）

#### HCN保持（復帰あり）

- `Active` 中に `Release` が発生した場合
  - 即時に `HoldBroken` 相当のイベントを通知（コンボ切れ）
  - ノート追跡は継続し、以後の `Press` で `Recovered` → `Active` に復帰可能
- 復帰の効力
  - 復帰時刻以降の要求のみ満たせる（切れていた区間の保持を遡及しない）

#### HCN終点

- CNと同様に、終点窓内でOFF状態になることが必要
- 終点窓を過ぎた場合は `MISS` 確定（以後の入力では救済しない）

### 5.4 判定ウィンドウ（IIDX標準準拠）

- 本プロジェクトの全判定ウィンドウ（Tap/CN/HCN/スクラッチ各要件）は **IIDX標準仕様に準拠**する
- 数値（us）を本書に固定値として埋め込むか、`RunnerConfig` のプリセット（例: `IidxStandard`）として保持するかは実装設計の範囲だが、**テストはそのプリセットに対して行う**

> 重要: 外部資料の引用や数表のコピペは不要。ここでは「Runnerが何に準拠するか」を仕様として固定する。

## 6. テスト戦略（実装前）

### 6.1 最低限のユニットテスト粒度（MVP）

- Tap 1ノート
  - 窓内 `Press` で OK
  - `deadline` 超過で MISS
  - 早い/遅いの境界（±1us）

- CN 1ノート
  - start成功 → 保持 → end窓 `Release` でOK
  - start成功 → 途中 `Release` で即MISS（復帰なし）
  - start成功 → end窓を過ぎても `Release` 無しでMISS

- HCN 1ノート
  - 途中 `Release` → Broken通知 → 再 `Press` で復帰 → end窓 `Release` で完了
  - 切れている間に end 到来 → MISS

- MSS 1ノート
  - checkpoint窓内に逆回転入力 → checkpoint OK
  - checkpoint未達 → そのcheckpointは BAD/POOR 確定、以後のcheckpointは通常通り判定できる（復活）

- BGM event 1つ
  - `time_us` 到達で `BgmTrigger` が1回だけ出る
  - `now_us` が飛んでも重複しない

### 6.2 時間窓をテスト可能にする設計

- `RunnerConfig`（仮）として時間窓・閾値を注入できること
  - 例: `tap_window`, `cn_start_window`, `cn_end_window`, `rev_window`, `spin_gap_us`, `scratch_min_abs_delta`, `scratch_dir_threshold`, `scratch_move_accumulation_us`, `scratch_ticks_per_rotation`
- テストでは小さな値を与え、境界の再現性を高める

### 6.3 ログ出力とテスト方式（CLIのみでも壊れない設計）

RunnerをCLIログ用途だけで開始する場合でも、将来のUI/可視化に拡張しやすく、かつテストが安定するように以下を推奨する。

- Runnerの一次出力は **構造化イベント（`RunnerEvent`）** とし、CLIログはそれを整形する「フォーマッタ」層で生成する
  - これにより、判定ロジックのテストは `RunnerEvent` の列比較で行える
- 文字列ログに対するテストを行う場合は、既存プロジェクトの方針に合わせて
  - 改行正規化（`\r\n` → `\n`）
  - パス等の環境差は末尾一致/正規化
  - I/Oエラーメッセージは prefix のみ固定
  といった戦略を採用する
- ログフォーマットを固定したい場合は、フォーマッタ出力のゴールデンテストを用意し、変更時は意図的に更新する運用にする

## 7. MVP と拡張案

### 7.1 MVP（今回確定する範囲）

- 入力: `MdfChart` + `TimedInputEvent`（キーON/OFF + スクラッチdelta）
- 出力: 判定イベント列 + BGM通知 + （任意で）スナップショット
- 判定:
  - Tap/CN/HCN/BSS/HBSS/MSS/HMSS
  - 判定ランクは `F-GREAT, P-GREAT, GREAT, GOOD, BAD, POOR`
    - `F-GREAT` は **誤差0us**（`timing_error_us == 0`）の判定
- 窓/閾値: Runner側定数（ただしConfig注入でテスト可能）

### 7.2 拡張案（互換を壊さず追加）

- 判定ランクの多段化（例: GREAT/GOOD/MISS 等）
- スコア/ゲージ（HCNの回復などを数値化）
- Seek/巻き戻し（状態再構築）
- スクラッチ入力の較正（delta→方向推定のパラメータ化）

## 8. 互換性とバージョニング（合意済み）

- MSS checkpointの復活仕様: 失敗したcheckpointの結果は BAD/POOR として残しつつ、以後のcheckpointは通常通り判定する
- `.mdf` の互換性: fail-fast（未知/非対応 `meta.version` はエラー）で進める

### 8.1 `meta.version` をRunnerが見るメリット（提案）

Runner側の分岐（複数実装）まで今すぐ行う必要はない一方で、`meta.version` を**完全に無視**すると「仕様が変わった `.mdf` を誤って読み、静かに判定がズレる」リスクが残る。

最小のメリット（=コストが小さい割に効く）として、次を提案する。

- Runner初期化時に `meta.version` を検査し、想定外なら明示的にエラー（fail-fast）
  - 例: `2.2` のみ許可、将来 `2.3` が出たら意図して対応追加

これにより、互換性問題が「静かな誤判定」ではなく「明確な失敗」として検出でき、トレーニング用途（入力の正確性計測）に対して安全側になる。

### 8.2 fail-fast採用の決定経緯（明文化）

本プロジェクトでは、後方互換性や将来拡張を考える上で「Runnerが解釈できる `.mdf` を出力すること」は基本的にコンパイラの責務とする。
一方で、`.mdf` の入力経路は将来にわたりコンパイラに限定されるとは限らない（例: 古い生成物の混在、手編集、外部ツール、将来の別コンパイラ/エディタ）。

トレーニングツールとして最重要なのは「正しく入力できたか」を正確に計測することであり、**仕様が食い違うデータを“それっぽく解釈してしまう”ことによる静かな誤判定**は致命的になりうる。

このため、互換性戦略として次を採用する。

- コンパイラは Runner が解釈可能な `meta.version` を持つ `.mdf` を出力する（出力互換の管理）
- Runnerは `meta.version` を検査し、未知/非対応の場合は **fail-fastでエラー**とする（静かな誤判定を防止）

将来 `meta.version` を増やす場合は、
1) Runner側で対応versionを追加する、または
2) コンパイラ側で特定versionへターゲット出力（必要ならダウングレード）する
のいずれかを、破壊的変更を避けつつ選択する。

## 9. 決定事項リスト（確定サマリ）

1. 判定ランク: `F-GREAT, P-GREAT, GREAT, GOOD, BAD, POOR` を採用する
2. 判定ウィンドウ: 全てIIDX標準仕様に準拠する
3. CN/HCN start: 開始時点ONを許容する（窓内でON状態になれば開始成立）
4. CN/HCN end: ウィンドウ内OFFを許容する（窓内でOFF状態になれば終点成立）
5. スクラッチ入力抽象: `delta` を受け取る方針で進める
6. 回転停止判定: 逆回転は停止判定に含める
7. MSS checkpoint失敗: 1つ失敗しても、次checkpointを正しく満たせばそこから判定が復活する
8. 処理優先度: 入力の処理を最優先（同時刻では入力→OK確定→deadline失敗確定の順）
9. checkpoint同時刻: 現時点の譜面では発生しない（将来DP拡張では起こりうる）

補足（互換性）:
- Runnerは `meta.version` による複数実装（分岐）を必須としない
- ただし **fail-fast方針**として、Runnerが想定していない `meta.version` の `.mdf` を入力した場合は明示的にエラーとする（静かな誤判定を防ぐ）
