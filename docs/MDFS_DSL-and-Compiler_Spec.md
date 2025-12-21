# 1. プロジェクト概要・アーキテクチャ定義

**Project:** BMS Mechanical Training Toolkit
**Codename:** "Precision"
**Version:** 2.2 (Corrected & Mixed Logic)

## 1.1 設計哲学 (Core Philosophy)
1.  **Absolute Time (絶対時間):**
    * 小節・拍という「音楽的概念」を排除し、全てを `マイクロ秒 (us)` で管理する。

2.  **Flat & Atomic (フラットかつ自己完結):**
    * **通常ノーツ**も**ロングノート**も、Runnerにとっては等しく「時間順に並んだイベント」として扱う。
    * CNは始点と終点を1つのオブジェクトに内包し、再生時のペアリング計算を不要にする。

## 1.2 データフロー
システムは以下の3層で構成される。

1.  **Human Layer (`.mdfs`):**
    * **Matrix Input**: 従来BMSのようなアスキーアート形式で、TapとCNを混在させて記述する。
    * **Toggle Logic**: CNは「開始」と「終了」を同じ文字で記述する。

2.  **Compiler Layer (`mdfs_compiler`):**
    * **Hybrid Parsing**: 通常ノーツは即時生成、CNはバッファリングして終点待ちを行うハイブリッドなパースを行う。

3.  **Machine Layer (`.mdf`):**
    * 物理演算（BPM/ScrollRate計算）済みのフラットなリスト。

```rust
// mdf_schema/src/lib.rs

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

// --- Events ---

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
    pub time_us: Microseconds, // 始点
    pub col: u8,               // レーン (0-7)
    #[serde(flatten)]
    pub kind: NoteKind,
    pub sound_id: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type")]
pub enum NoteKind {
    /// 通常ノーツ (Tap)
    #[serde(rename = "tap")]
    Tap,
    
    /// チャージノート (Charge Note)
    #[serde(rename = "cn")]
    ChargeNote { end_time_us: Microseconds },
    
    /// ヘルチャージ (Hell Charge Note)
    #[serde(rename = "hcn")]
    HellChargeNote { end_time_us: Microseconds },
    
    /// バックスピンスクラッチ (Back Spin Scratch)
    /// 始点〜終点のホールドを持つスクラッチ。
    /// 逆方向入力は「終点」で必須（判定はランナー側）。
    #[serde(rename = "bss")]
    BackSpinScratch { end_time_us: Microseconds },

    /// ヘル・バックスピンスクラッチ (Hell Back Spin Scratch)
    /// BSSの要件 + ヘルチャージ特性（途中で切れても復帰可能）。
    #[serde(rename = "hbss")]
    HellBackSpinScratch { end_time_us: Microseconds },

    /// マルチスピンスクラッチ (Multi Spin Scratch)
    /// 始点〜終点のホールドに加えて、中間チェックポイントを持つスクラッチ。
    /// 各チェックポイントおよび終点で、逆方向への入力が必須（判定はランナー側）。
    #[serde(rename = "mss")]
    MultiSpinScratch {
        end_time_us: Microseconds,
        /// 逆方向入力が要求される中間チェックポイント（絶対時間）。
        /// DSL側の `@rev_every` / `@rev_at` で生成される。
        #[serde(default)]
        reverse_checkpoints_us: Vec<Microseconds>,
    },

    /// ヘル・マルチスピンスクラッチ (Hell Multi Spin Scratch)
    /// MSSの要件 + ヘルチャージ特性（途中で切れても復帰可能）。
    #[serde(rename = "hmss")]
    HellMultiSpinScratch {
        end_time_us: Microseconds,
        #[serde(default)]
        reverse_checkpoints_us: Vec<Microseconds>,
    },
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
pub struct BgmEvent {
    pub time_us: Microseconds,
    pub sound_id: String,
}
```

補足:
* `VisualEvent` の `beat_n` / `beat_d` は**表示（ガイド目盛り）用**であり、譜面の判定要件やノーツ生成ロジックの根拠として利用しない。

# 2. 判定仕様: チャージ系ノーツ前提 (Judgement Assumptions)

この節は「譜面データが表現するもの（開始/終了/中間）」と「ランナーが判定するルール」を明確化する。

## 2.1 通常チャージノート (CN)

* CNは **始点と終点** を持つ。
* CNは、始点〜終点の間でボタンが **OFF になるとコンボが切れる**（MISS判定となり、そのCNは復帰しない）。
* CNは、終点でボタンを **OFF（離す）** できない場合もMISS判定となる。

## 2.2 ヘルチャージノート (HCN)

* HCNは **始点と終点** を持つ。
* HCNは、中間でボタンがOFFになった場合 **コンボが切れる**。
    * ただし、再度ボタンがONになった場合、そこから再び判定が有効となる（復帰可能）。
    * 復帰は「再入力が始まって以降の要求」に対してのみ有効とする（切れていた区間に遡って要求を満たすことはできない）。
* HCNは、押している間ゲージが回復する。
    * 途中で切れても押しなおせば、回復/判定が再開する。

## 2.3 BSS (Back Spin Scratch)

* BSSは **始点と終点** を持つ（中間チェックポイントは持たない）。
* BSSは、中間でスクラッチの回転を止めると **コンボが切れる**。
* BSSは、一度コンボが切れると再度スクラッチを回しても **判定は復活しない**。
* BSSは、終点でスクラッチを **逆回転** させる必要がある。

### 回転の定義（BSS/HBSS）

スクラッチ入力はロータリーエンコーダーのような入力を想定し、BSS/HBSSにおける回転継続/停止を次のように定義する。

* 「回転が続いている」= 一方向の入力が継続している状態。
* 「回転が停止した」= 入力が止まった、または回転方向が逆になった状態。
    * ただしBSS/HBSSの終点では、逆回転は別要件として評価される（終点での逆回転は停止扱いによる失敗とは別枠）。

## 2.4 MSS (Multi Spin Scratch)

* MSSは **始点・中間チェックポイント・終点** を持つ。
* MSSは、中間チェックポイントごとにスクラッチを **逆回転** させる必要がある。
* MSSは、終点でスクラッチを **逆回転** させる必要がある。

## 2.5 ヘル版（HBSS / HMSS）

* BSS/MSS には、それぞれ **ヘルチャージノートの特性を持つバージョン**が存在する。
    * HBSS: BSSの要件に加え、途中で切れても再度回すことで判定が復帰する。
    * HMSS: MSSの要件に加え、途中で切れても再度回すことで判定が復帰する。
* 復帰の有効範囲
    * HBSS/HMSSの「復帰」は、再入力が始まって以降の要求に対してのみ有効とする（復帰前の未達要件を遡って満たすことはできない）。
    * HMSSにおいて、入力が切れている間に到来した中間チェックポイントは「未達として保持」せず、復帰後の判定対象にはしない（復帰後は復帰時刻以降に到来するチェックポイントのみ判定する）。
    * HMSSの終点要件（終点での逆回転）は、終点到来時点で入力が有効な場合に判定される。

# 3. 記述言語: MDFS DSL Specification

## 3.1 記述ルール (Body)

`track: |` セクションでは、8文字の文字列で1行（1ステップ）を表現します。
フォーマット: `S1234567` (Index 0=Scratch, 1-7=Keys)

### 時間の進行 (Absolute Time)

* `@bpm` はトラック中の任意位置で変更が起こりえ、**宣言された行から即時に有効**になる。
* 1行(1ステップ)の長さは `@div` とその時点の `@bpm` によって決まり、各行の処理後に `current_time_us` を加算する。
* したがって、BPM変更は **次の行の時間刻みに反映**される。

* `@bpm` または `@div` が未設定のままノーツ行が出現した場合はエラー。(E3001, E3002)
* `@bpm` / `@div` の値が不正（0以下等）な場合はエラー。(E3003, E3004)

### 時間計算（us）と丸め規則

本仕様は内部表現として `u64` の `time_us` を採用する。
一方で、`@bpm` は小数を許容するため、ステップ長は理論上 `f64` を含む。
実装差で譜面がズレないよう、**丸め規則**を以下に固定する。

* 1ステップの理論時間（秒）
    * `step_duration_sec = (60.0 / bpm) * (4.0 / div)`
* 1ステップの理論時間（us）
    * `step_duration_us_f64 = step_duration_sec * 1_000_000.0`
* `us` への変換
    * **四捨五入（0.5は切り上げ）**で `u64` に変換する。
    * 例: `step_duration_us = floor(step_duration_us_f64 + 0.5) as u64`
* 累積誤差の扱い
    * **Pass 1（Time Map Pass）では「ノーツ行（8文字ステップ）」の開始時刻を、上記丸め規則に従って逐次加算で確定**する。
    * これにより、以後の計算（`@rev_at` 等）は開始時刻テーブル参照に統一され、BPM変化を含んでも解釈が一意になる。

* Pass 1 の時刻計算で `u64` の範囲を超える場合はエラー。(E3005)

### 行の文法（空白・コメント）

`track: |` の本文は「行」の列からなる。

* 空行は無視してよい。
* 行頭/行末の空白は許容し、解析時は適宜トリムしてよい。
* コメント行
    * 先頭の非空白文字が `#` の行はコメント行として無視する。
* インラインコメント
    * ノーツ行/ディレクティブ行の末尾に `#` が現れた場合、その `#` 以降はコメントとして無視する。
    * これにより、例にある `: SOUND_SPEC  # ...` のような注釈を許容する。
* ディレクティブ行
    * `@bpm ...` / `@div ...` / `@...` はディレクティブ行であり、**ステップ（時間の進行）としてカウントしない**。
    * `@sound_manifest <path>` はキー音マニフェスト（JSON）を指定するディレクティブである。
        * 例: `@sound_manifest sounds.json`
        * `<path>` は `.mdfs` ファイルからの相対パス、または絶対パスを許容する（実装が相対パスのみ対応でもよいが、その場合は仕様として制限を明記する）。
        * `@sound_manifest` は `track: |` の開始前に書くことを推奨し、コンパイラは最初に出現した時点で読み込む。
        * 読み込みに失敗した場合や、JSONが不正な場合はコンパイルエラーとする。(E2001, E2002)
        * マニフェスト内容を検証する実装では、値が不正（非文字列/空等）の場合もコンパイルエラーとしてよい。(E2003)
        * 同一ファイル内で複数回指定された場合はコンパイルエラーとする（曖昧さ回避）。(E2004)
* ノーツ行（ステップ行）
    * 先頭8文字が `S1234567`（レーン0-7）であり、これが **1ステップ**を表す。
    * 9文字目以降は任意の「行末メタ情報」であり、現行仕様では以下を解釈する。
        * `: SOUND_SPEC`（任意）
        * `@rev_every` / `@rev_at`（任意、MSS/HMSS開始行のみ）

### ノーツ文字定義
各レーンの文字によって生成されるノーツが決まります。

* **通常ノーツ (Tap)**
    * `N` : 全レーン共通で Tap として扱う（推奨）。
    * `S` : スクラッチレーン(Col 0)専用の Tap 表記（**スクラッチは `S` を使うことを推奨**：可読性のため）。

* **特殊ノーツ (Hold/Special)**
    * `l` : **Charge Note** (Start/End Toggle)
    * `h` : **Hell Charge Note** (Start/End Toggle)
    * `b` : **Back Spin Scratch** (Start/End Toggle, Col 0 専用)
    * `m` : **Multi Spin Scratch** (Start/End Toggle, Col 0 専用)
    * `B` : **Hell Back Spin Scratch** (Start/End Toggle, Col 0 専用)
    * `M` : **Hell Multi Spin Scratch** (Start/End Toggle, Col 0 専用)

* **その他**
    * `.` : 空白 (休符)
    * `!` : **MSS/HMSS 中間チェックポイント** (Col 0 専用, MSS/HMSSホールド中のみ有効)

### 予約語（未定義文字）

`track: |` 本文において、各ステップ行の **先頭8文字（S1234567）** に出現できる文字は、以下に限る。

* `.` / `N` / `S` / `l` / `h` / `b` / `m` / `B` / `M` / `!`

上記以外の文字は **予約語（未定義文字）** とし、`track: |` 本文の「先頭8文字」に出現した場合はコンパイルエラーとする。(E4001)

補足:
* `: SOUND_SPEC` や `@rev_every` / `@rev_at` などの「行末のメタ情報」部はこの制約の対象外（任意文字列）であり、予約語判定は **先頭8文字にのみ適用**する。

### SOUND_SPEC（サウンド指定）

本節では「譜面上のノーツ/BGMイベントが参照するサウンドID」と、その解決方法を定義する。

#### キー音定義（外部マニフェスト）

譜面（`.mdfs`）とは別に、サウンドIDとファイルパスの対応を定義したマニフェストを用意できる。

* マニフェストはJSONを推奨する。
* 例（推奨フォーマット: 文字列→文字列のマップ）:

```json
{
    "K01": "kick1.wav",
    "K02": "kick2.wav",
    "S01": "scratch.wav",

    "S_LP": "scratch_long.wav",
    "S_MS": "scratch_mss.wav",
    "S_MS2": "scratch_mss2.wav",
    "S_MS3": "scratch_mss3.wav",

    "SE_END": "se_end.wav",
    "SE_CP": "se_checkpoint.wav"
}
```

* コンパイラは `.mdfs` の先頭でマニフェストを読み込み、出力 `.mdf` の `MdfChart.resources` に同等のマップとして格納してよい。
    * `Note.sound_id` / `BgmEvent.sound_id` は、このマップのキーを参照する。
* マニフェストに存在しないIDが譜面側から参照された場合はコンパイルエラーとする。(E2101)
* マニフェストの指定方法
    * `.mdfs` に `@sound_manifest <path>` を記述し、そのJSONを読み込む。
    * `@sound_manifest` が省略された場合、`MdfChart.resources` は空マップでもよい（この場合、譜面からサウンドIDを参照したらエラー）。(E2101)

#### mdfs側の指定形式（SOUND_SPEC）

`track: |` のノーツ行末では `: SOUND_SPEC` により、そのステップで鳴らす（またはノーツへ付与する）サウンドIDを指定できる。

* `SOUND_SPEC` は次のいずれかの形式を取りうる。
    * 単一指定（`SOUND_ID`）
    * レーン別指定（8スロット配列）
    * 無指定（省略または空配列）

* 上記以外の形式はエラー。(E1001)

#### 1) 単一指定（従来互換）

* `: SOUND_ID`
    * その行で生成された全ノーツに同じ `sound_id` を付与する。
    * 同時押し（同一ステップで複数レーンにノーツがある）の場合も、各ノーツに同じ `sound_id` が付与される。

#### 2) レーン別指定（8スロット）

* `: [S0,S1,S2,S3,S4,S5,S6,S7]`
    * `S0` はスクラッチ（col=0）、`S1..S7` は鍵盤（col=1..7）に対応する。
    * 各スロットは **サウンドID** または **`-`（指定なし）** を取る。
    * スロット数が8でない場合はエラー。(E1002)
    * スロットのトークンが不正（例: 空要素や未定義トークン）の場合はエラー。(E1003)
    * この形式が使われた場合、生成されたノーツには **同じレーンのスロット値**を `sound_id` として付与する（`-` は `None` 相当）。
    * 例: `S..N..N. : [S01,-,-,K01,-,-,K02,-]`
        * col0 のTapに `S01`、col3 のTapに `K01`、col6 のTapに `K02` が付与される。

#### 3) 無指定（省略 or 空配列）

* `: SOUND_SPEC` 自体を省略する、または `: []` を指定した場合、そのステップでのサウンド指定は「全レーン無指定」とする。
    * 同一ステップにノーツが存在しても、`sound_id` は付与されない（`None`）。

#### SOUND_SPEC の例外ルール（終点/中間点/無音ステップ）

* CN/HCN の終点行に指定された `SOUND_SPEC` は無視される（音は始点に紐づく）。
* BSS/MSS（および HBSS/HMSS）の終点行に `SOUND_SPEC` が指定されている場合
    * ノーツ（ホールド本体）の `sound_id` へは付与しない（ホールドは始点のノーツに紐づくため）。
    * 代わりに、`BgmEvent` を生成して「終点で鳴る音」を表現する。
        * 単一指定（`: SOUND_ID`）の場合: `BgmEvent { time_us: 終点行のステップ開始時刻, sound_id }` を1つ生成する。
        * レーン別指定（`: [..]`）の場合: **非 `-` スロットごとに** `BgmEvent` を生成してよい（同時に複数SEを鳴らせる）。
* MSS/HMSS の `!` 行に `SOUND_SPEC` が指定されている場合
    * `!` 自体はノーツではないが、`BgmEvent` を生成して「中間点で鳴る音」を表現できる（生成規則は上記と同様）。
* さらに、**ノーツを一切生成しないステップ行**（例: `........ : SE01`）に `SOUND_SPEC` が指定されている場合も、
  `BgmEvent` を生成して「任意のタイミングで鳴る音」を表現してよい（生成規則は上記と同様）。

* MSS の追加指定（例: `@rev_every` / `@rev_at`）は、`SOUND_SPEC` の後ろに続けて記述できる。

### BSS / MSS 判定要件

* 判定前提は「# 2. 判定仕様: チャージ系ノーツ前提」に従う。
* `.mdf` / `.mdfs` は「長さ・中間チェックポイント」を表現するのみで、回転方向や回数といった入力の詳細は持たない。
* 逆方向入力や回転停止の検出など、入力の解釈と判定は **ランナー側の責務**とする。

#### 逆方向入力チェックポイント (MSS/HMSS)

「ホールド中に指定した箇所で逆回転させる」ために、MSS/HMSS には逆方向入力の中間チェックポイント（要求時刻）を持たせる。

* `.mdf` では `reverse_checkpoints_us: Vec<Microseconds>` に **絶対時間(us)** の一覧として格納する。
* ランナーは、各チェックポイントについて「その近傍で逆方向の入力が発生した」ことを判定する。
    * 判定窓（許容誤差）はランナー側定数として扱い、チャートには含めない。
* チェックポイントの生成元は以下を想定する。
    * MSS開始行の `@rev_every` / `@rev_at`
    * HMSS開始行の `@rev_every` / `@rev_at`
    * トラック本文中の `!` マーカー（視覚的指定）

#### DSL指定（周期/任意/マーカー）

MSS/HMSSホールド開始行（`m` または `M` が出現した行）の末尾に、以下を追記できる。

* これら（`@rev_every` / `@rev_at` / `!`）が MSS/HMSS 以外の文脈で指定された場合はエラー。(E4201)

※ `N` や `@rev_at` のカウント対象は「8文字のノーツ行(ステップ)」であり、`@bpm` / `@div` 等のディレクティブ行やコメント行は含めない。
※ MSS/HMSS におけるステップ番号は **ホールド開始行（`m`/`M` が出現した行）を 1** として数える。

* `@rev_every N`
    * `N` は「行(ステップ)単位」の間隔。
    * `N` が非整数または1未満の場合はエラー。(E1005)
    * 例: `@div 16` のとき、4分音符刻みは 16分音符×4 なので `@rev_every 4`。
    * コンパイラは **Pass 1（Time Map Pass）の「ノーツ行開始時刻テーブル」**を参照して、ホールド中に `N` 行ごとの中間チェックポイントを生成する。
        * ステップ番号で表すと、チェックポイントは `1 + N`, `1 + 2N`, `1 + 3N` ...（終点は除外）となる。
        * `hold_start_step_index` を「ホールド開始ノーツ行のステップインデックス（ノーツ行のみで数える）」とすると、
          ステップ番号 `s` に対応するチェックポイント時刻は `step_start_time_us[hold_start_step_index + (s - 1)]` とする。

* `@rev_at a,b,c`
    * `a,b,c` は「ホールド開始行を 1 としたステップ番号」（**2以上の整数**）のリスト。
    * リストが不正（非整数/2未満/空など）の場合はエラー。(E1004)
    * 例: `@rev_at 4,7,12` のように、複合的な刻みにも対応できる。
    * コンパイラは指定ステップに対応する絶対時間(us)を計算し、中間チェックポイントとして格納する。
        * **Pass 1（Time Map Pass）で作成した「ノーツ行開始時刻テーブル」を参照して決定する。**
        * `hold_start_step_index` を「ホールド開始ノーツ行のステップインデックス（ノーツ行のみで数える）」とすると、
          `a` に対応するチェックポイント時刻は `step_start_time_us[hold_start_step_index + (a - 1)]` とする。
        * これにより、ホールド中のBPM変更があっても `@rev_at` の解釈が一意になる。

* `!`（視覚マーカー）
    * MSSホールド中に、スクラッチレーン(Col 0)へ `!` を置くと、そのステップ時刻が中間チェックポイントになる。
    * `!` はノーツではなく「逆方向入力が必要な箇所」の指定であり、音やノーツ生成は行わない。
    * `!` は **MSS/HMSSホールド中のみ有効**。BSS/HBSS中または未ホールド時に出現したらエラー。(E4003, E4102)
    * `!` が存在する行でも、他レーン(Col 1-7)の同時押しやチャージノート記述は通常通り有効。

* 同時指定（集合化）
    * `@rev_every` / `@rev_at` / `!` は併用でき、各指定から生成された中間チェックポイントを **集合化（重複除去）**して採用する。
    * 重複判定は `us` の**完全一致**で行う。
    * 生成後は時刻順（昇順）にソートする。
    * `end_time_us` と同一時刻のチェックポイントが生成された場合は除外する（終点は別要件として扱う）。

* 終点について
    * MSS/HMSS は終点でも逆方向入力が必須だが、終点は `end_time_us` として既に存在するため、`reverse_checkpoints_us` には含めない（ランナー側は「全中間チェックポイント + 終点」を判定対象とする）。

## 3.2 記述例 (Mixed Pattern)

通常ノーツのリズムの中に、CNが混ざる実践的な例です。

```text
@sound_manifest sounds.json
track: |
    @bpm 150
    @div 16

    # --- Measure 1 (Standard Rhythm) ---
    # 基本的な8ビート
    S....... : [S01,-,-,-,-,-,-,-]
    ..N..... : [-,-,K01,-,-,-,-,-]
    S....... : [S01,-,-,-,-,-,-,-]
    ..N..... : [-,-,K01,-,-,-,-,-]

    # --- Measure 2 (Mixed CN) ---
    # 左手(Col 1)でCNを押しながら、右手(Col 7)でTapを刻む

    .l.....N : [-,L01,-,-,-,-,-,K01]   # <--- Col 1: CN Start, Col 7: Tap (レーン別SE)
    .......N :       # <--- Col 1: Holding,  Col 7: Tap
    .......N :       # <--- Col 1: Holding,  Col 7: Tap
    .l.....N :       # <--- Col 1: CN End,   Col 7: Tap

    # --- Measure 3 (Complex) ---
    # 皿(Col 0)のCNと、鍵盤のTap

    b..N.... : [S_LP,-,-,K01,-,-,-,-]  # <--- BSS Start (Col 0) + 同時Tap
    ...N.... :
    ...N.... :
    b....... : [SE_END,-,-,-,-,-,-,-]  # <--- BSS End (Col 0) 終点SEはBgmEvent
    
    # --- Measure 4 (Multi Spin Scratch) ---
    # MSSは譜面データ上は「長さを持つスクラッチ」として表現し、
    # 中間チェックポイントおよび終点で逆方向入力が必要（判定はランナー側）。    
    m....... : [S_MS,-,-,-,-,-,-,-] @rev_every 4  # <--- MSS Start (Col 0)
    ........ :
    ........ :
    m....... : [SE_END,-,-,-,-,-,-,-]  # <--- MSS End (Col 0) 終点SEはBgmEvent

    # --- Measure 5 (Complex MSS: Directives) ---
    # 周期(4分刻み) + 任意指定(複合)を同時に使う例（集合化）。
    # 例: @rev_every 4 -> 5,9,13... に加え、@rev_at 8,13 を追加。   
    m....... : [S_MS2,-,-,-,-,-,-,-] @rev_every 4 @rev_at 8,13
    ........ :
    ........ :
    ........ :
    ........ :
    ........ :
    ........ :
    ........ :
    ........ :
    ........ :
    ........ :
    ........ :
    m....... :

    # --- Measure 6 (Complex MSS: Visual Markers) ---
    # `!` を置いた行が中間チェックポイントになる（複合的な刻みを視覚的に表現）。
    # この例では 5,8,13 ステップ目に `!` を置いている。 
    m....... : [S_MS3,-,-,-,-,-,-,-]
    ........ :
    ........ :
    ........ :
    !....... : [SE_CP,-,-,-,-,-,-,-]   # step 5 (中間点SEはBgmEvent)
    ........ :
    ........ :
    !....... : [SE_CP,-,-,-,-,-,-,-]   # step 8
    ........ :
    ........ :
    ........ :
    ........ :
    !....... : [SE_CP,-,-,-,-,-,-,-]   # step 13
    m....... :
```

## 3.3 コンパイル結果例（.mdf 抜粋）

以下は「MDFS入力の一部」と、それをコンパイルしたときに得られる `.mdf` の `notes` 抜粋例です（説明のための最小例）。

前提:

* `@bpm 150` / `@div 16` のとき、1ステップ = 16分音符 = 100,000us
* `@rev_at` のステップ数は「ホールド開始行（`m`）を 1」とする（例: `@rev_at 3` は始点 + 2ステップ）

入力（抜粋）:

```text
@sound_manifest sounds.json
track: |
    @bpm 150
    @div 16

    m....... : [S_MS,-,-,-,-,-,-,-] @rev_at 3
    ........ :
    !....... : [SE_CP,-,-,-,-,-,-,-]
    ........ :
    m....... :
```

出力（`notes` 抜粋イメージ）:

```json
[
    {
        "time_us": 0,
        "col": 0,
        "type": "mss",
        "end_time_us": 400000,
        "reverse_checkpoints_us": [200000],
        "sound_id": "S_MS"
    }
]
```

### 4. コンパイラロジック (Corrected Logic)

通常ノーツの即時生成と、CNの遅延生成を分岐するロジックです。

# 4. コンパイラロジック (Compiler Logic)

**注意:** `@rev_every` 等の「行数ベースの時間計算」と「途中でのBPM変更」を正しく両立させるため、
コンパイラは **2パス（Two-Pass）処理** を必須とします。
1. **Time Map Pass:** 全行を走査し、各行（ステップ）の絶対開始時刻を計算・キャッシュする。
2. **Generation Pass:** ノーツ生成を行う。`@rev_every` の計算には Pass 1 で計算した時刻マップを参照する。

**Crate:** `mdfs_compiler`

## 4.1 コンテキスト

```rust
struct CompilerContext {
    current_time_us: u64,
    // ... (bpm, rate settings)
    
    notes: Vec<Note>, // 出力先
    
    // CN待機バッファ
    // Key: Lane(0-7), Value: 始点ノーツ
    pending_cn: HashMap<u8, Note>, 
}
```

## 4.2 行解析ループ (Main Loop)

1行の文字列（例: `.l.....N`）を `chars().enumerate()` で回し、インデックス `i` (Col) に応じて処理する。

### Logic Flow

1. **文字 `c` が `.` (Dot) の場合:**
   * 何もしない (Continue)。

2. **文字 `c` が `!` の場合:**
    * MSS/HMSSホールド中(Col 0 で `pending_cn` が MSS/HMSS)であれば、中間チェックポイントとして `current_time_us` を記録する。
    * この行に `: SOUND_SPEC` が指定されている場合、`BgmEvent` を生成してよい（`### SOUND_SPEC` の規則に従う）。
    * それ以外（未ホールド / BSS中 / Col 0 以外）はエラー。(E4003, E4102)

3. **文字 `c` が Tap文字 (`N`, `S`) の場合:**
    * **即時生成:** `Note { time_us: current, col: i, kind: Tap }` を生成。
      * 行末に指定された `SOUND_SPEC` を解釈し、該当ノーツへ `sound_id` を付与する。
      * `ctx.notes.push(note)`。

4. **文字 `c` が CN文字 (`l`, `h`, `b`, `m`, `B`, `M`) の場合:**
   * **Check Pending:** `ctx.pending_cn` に `col: i` があるか確認。
   
   * **Case A: 無い (Start)**
     * 始点ノーツを作成。`kind` は仮の状態（または専用のPending状態）にする。
         * 行末の `SOUND_SPEC` を解釈し、始点ノーツへ `sound_id` を付与する。
     * `ctx.pending_cn.insert(i, note)`。
   
   * **Case B: ある (End)**
     * `ctx.pending_cn.remove(i)` で始点ノーツを取り出す。
     * 始点ノーツの `kind` を確定させる（例: `ChargeNote { end_time_us: current }`）。
     * **注意:**
         * CN/HCN の終点行に `SOUND_SPEC` があっても無視する（CN/HCNの音は始点に紐づく）。
         * BSS/MSS（およびヘル版）の終点行に `SOUND_SPEC` がある場合、ノーツには付与せず `BgmEvent` を生成してよい（終点で鳴る音）。
     * `ctx.notes.push(completed_note)`。

4.5 **補足（無音ステップのSOUND_SPEC）:**
    * そのステップでノーツが一切生成されず、かつ `: SOUND_SPEC` が指定されている場合、`BgmEvent` を生成してよい。

5. **バリデーション:**
    * 同じ場所（時間・レーン）にTapとCN始点が重なるような記述はエラーとする。(E4004)
     * `S` および `b`、`m`、`B`、`M` はスクラッチレーン(Col 0)以外に出現したらエラーとする。(E4002)
     * `!` はスクラッチレーン(Col 0)以外、または MSS/HMSSホールド中以外に出現したらエラーとする。(E4003)

6. **エラー情報（最低要件）:**
    * コンパイルエラーは、最低限以下を含めること。
        * 行番号（入力ファイル上の行番号）またはステップ番号（ノーツ行のみで数えたインデックス）
        * `col`（0-7）
        * `time_us`（可能であれば）
        * 問題の文字（例: 予約語、`!` の不正位置、未クローズのトグルなど）

    * 可能であれば、本仕様の「# 6. エラー定義（Error Codes）」に従い、`code`（エラーコード）と `message`（説明）も付与する。

# 5. 実装ロードマップ

## Phase 1: Core
* [ ] Schema Definition (`mdf_schema`)
* [ ] **Unit Test**: TapとCNが混在した `Vec<Note>` のJSONシリアライズ確認。

## Phase 2: Compiler Logic
* [ ] Parser: `nom` で行解析。
* [ ] **Logic**: 上記 4.2 の「Tap即時生成」と「CNトグル」の分岐処理実装。
* [ ] Validation: ファイル終端で `pending_cn` が残っていたらエラーを出す処理。(E4101)

## Phase 3: Viewer
* [ ] 単なるTapと、長さを持つCNが視覚的に区別されて描画されるか確認。

# 6. エラー定義（Error Codes）

本節は「コンパイルエラーとする」と記述された箇所について、**具体的なエラー表現（フォーマット）**と**エラーコード**を定義する。

## 6.1 エラーの出力フォーマット（推奨）

コンパイラは、エラーを以下の構造（JSON相当）で出力できることを推奨する。

```json
{
  "code": "E1001",
  "kind": "Parse" ,
  "message": "Unknown token in SOUND_SPEC: '...'.",
  "file": "chart.mdfs",
  "line": 42,
  "column": 5,
  "step_index": 17,
  "lane": 3,
  "time_us": 1700000,
  "context": "..N..... : [S01,-,-,K01,-,-,K02,-]",
  "help": "Use '-' for empty slots, or omit ': SOUND_SPEC'."
}
```

* 必須（推奨）
    * `code`: エラーコード（本節のテーブルを参照）
    * `kind`: 大分類（`Parse` / `Semantic` / `IO` / `TimeMap` / `Validation`）
    * `message`: 1行で原因が分かる説明
* 位置情報（可能な範囲で）
    * `file`, `line`, `column`: 入力ファイル上の位置
    * `step_index`: ノーツ行のみで数えたステップ番号（0-based推奨、または明記した上で1-basedでも可）
    * `lane`: 0-7（特定できる場合）
    * `time_us`: Pass 1 で確定した絶対時刻（特定できる場合）
* 補助
    * `context`: 問題の行（トリム済みでよい）
    * `help`: 修正ヒント（任意）

## 6.2 エラーコード表

規約:
* `E1xxx`: パース/構文（SOUND_SPEC含む）
* `E2xxx`: 入出力/外部リソース（マニフェスト）
* `E3xxx`: 時間マップ/ディレクティブ（BPM/DIV等）
* `E4xxx`: 譜面バリデーション（レーン制約/同時配置/トグル不整合等）

| Code | Kind | 条件（概要） | 最低限の付帯情報 |
|---|---|---|---|
| E1001 | Parse | `: SOUND_SPEC` の構文が不正（括弧/カンマ/配列形式など） | line, message, context |
| E1002 | Parse | レーン別配列が8スロットでない（`[S0..S7]` の要件違反） | line, message, context |
| E1003 | Parse | レーン別配列に `-` 以外の空要素/不正トークンがある | line, lane(可能なら), context |
| E1004 | Parse | `@rev_at` のリストが不正（非整数/2未満/空など） | line, message |
| E1005 | Parse | `@rev_every` の `N` が不正（非整数/1未満） | line, message |
| E1006 | Parse | 不明なディレクティブ（`@...`） | line, message |
| E1101 | Parse | ノーツ行の先頭8文字が不足/過剰、またはレーン文字列として解釈不能 | line, context |
| E2001 | IO | `@sound_manifest <path>` が読めない（存在しない/権限/パス不正） | file, line, message |
| E2002 | IO | マニフェストJSONが不正（JSONパース失敗） | file, line(可能なら), message |
| E2003 | IO | マニフェストの値が不正（空パス/非文字列など、実装が検証する場合） | file, message |
| E2004 | IO | `@sound_manifest` が複数回指定された | line, message |
| E2101 | Semantic | 譜面が参照したサウンドIDがマニフェストに存在しない | line, lane(可能なら), sound_id |
| E3001 | TimeMap | `@bpm` が未設定のままノーツ行が出現した | line, message |
| E3002 | TimeMap | `@div` が未設定のままノーツ行が出現した | line, message |
| E3003 | TimeMap | `@bpm` の値が不正（0以下/NaN/Infinity等） | line, message |
| E3004 | TimeMap | `@div` の値が不正（0以下） | line, message |
| E3005 | TimeMap | Pass 1 の時刻計算がオーバーフローした（`time_us` が `u64` 範囲外） | line, message |
| E4001 | Validation | 予約語（未定義文字）が先頭8文字に出現した | line, lane, char |
| E4002 | Validation | スクラッチ専用文字（`S`/`b`/`m`/`B`/`M`）が col0 以外に出現した | line, lane |
| E4003 | Validation | `!` が col0 以外、または MSS/HMSSホールド中以外に出現した | line, lane |
| E4004 | Validation | 同一（time_us, lane）に Tap とホールド始点が重複した | line, lane, time_us |
| E4101 | Validation | トラック終端でトグル（CN/HCN/BSS/MSS/HBSS/HMSS）が未クローズ | lane, start_line, start_time_us |
| E4102 | Validation | `!` が BSS/HBSSホールド中に出現した | line, lane |
| E4201 | Semantic | `@rev_every/@rev_at/!` が MSS/HMSS 以外の文脈で指定された | line, message |

注:
* `sound_id` / `char` / `start_line` などは、出力フォーマット上は `message` に含めてもよいが、機械処理を考えるなら独立フィールドとして持つことを推奨する。