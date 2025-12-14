# Oxidizer - Concept & Requirements

## 1. プロジェクト概要

* Oxidizer は、beatmania IIDXのプレイスキル向上に特化した、Rust製のリズムゲーム・トレーニングツール。
既存のBMSプレイヤーとしての機能に加え、 **「ミリ秒単位の精度の分析」と「練習用譜面の自動生成」** 機能を備える。

* ターゲットプラットフォームは Linux (Arch Linux等) を第一候補とし、Windows 11非対応の古いPC（Intel第4世代〜第7世代）を「極限の低遅延専用マシン」として再生させることを目指す。

## 2. コア・バリュー (Why Rust & Linux?)

* Zero GC Pauses: Rustの採用により、ガベージコレクションによる数ミリ秒のフレーム落ち（判定ズレ）を物理的に排除する。
* Audio Driven: 映像フレームではなく、音声サンプルカウントを絶対時間とする「Audio is God」アーキテクチャを採用。
* Linux Native Performance: WindowsのDWMやバックグラウンドプロセスをバイパスし、カーネルレベルでのリアルタイムスケジューリングとRaw Input取得を実現する。

## 3. 機能要件 (Functional Requirements)

### A. 再生・練習機能

* BMS再生: .bms, .bme, .bml 形式のサポート。  
* 再生速度制御: 0.5倍 〜 2.5倍速のピッチ補正付きタイムストレッチ再生。  
* メトロノーム/クリック: リズムキープ練習のための正確なガイド音。

### B. 分析・可視化

* ヒートマップ: 楽曲のタイムラインおよびボタン配置ごとのミスの傾向を可視化。  
* 判定ログ: すべての打鍵のズレ（ms）を記録し、統計データとして蓄積。

### C. 譜面生成 (The Generator)

* 無限練習モード: ユーザーの設定した密度・傾向（乱打、階段、同時押し等）に基づき、練習用譜面をリアルタイムに無限生成する。  
* 弱点克服: ヒートマップの分析結果に基づき、苦手なパターンを重点的に生成する。

## 4. 非機能要件 (Non-Functional Requirements)

* リフレッシュレート: 144Hz以上でのティアリングなし・スタッターなしの描画。
* 入力遅延: 1ms以下の入力ポーリング（1000Hz Polling）。
* 音声遅延: ALSA/PipeWireを用いた極小レイテンシ構成。
* ハードウェア: Intel Core i5 (4th Gen) + GTX 1050 程度の環境で安定動作すること。
    * 最低動作環境 (Minimum):
        * CPU: Intel Core i5 第4世代 (Haswell) / AMD FX Series
        * RAM: 4GB (DDR3)
        * GPU: Intel HD Graphics 4600 (CPU内蔵)
    * 推奨動作環境 (Recommended):
        * CPU: AMD Ryzen 3 2200G (APU) / Intel Core i3 第8世代
        * RAM: 8GB (DDR3/DDR4問わず)
        * GPU: AMD Radeon Vega 8 (APU内蔵) 程度
        
## 5. UI/UX方針

* 普段使いのLinuxデスクトップ（KDE Plasma等）と共存できるよう、ウィンドウモードおよびボーダーレスウィンドウをサポート。
* ゲーム起動時のみ GameMode 等を利用してプロセス優先度を最大化する。