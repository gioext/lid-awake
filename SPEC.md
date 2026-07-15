# Lid Awake - 実装仕様

## 概要

Lid Awakeは、MacBookの蓋を閉じたときのスリープ動作を`pmset disablesleep`で切り替えるmacOS専用メニューバーアプリである。対象はこの設定の表示とON/OFFだけとし、システム常駐コンポーネントは追加しない。

| 項目 | 値 |
|---|---|
| アプリ名 | Lid Awake |
| Bundle ID | `dev.gratchs.lidawake` |
| 最低対応OS | macOS 11 |
| 技術スタック | Tauri v2 / Svelte 5 / TypeScript / Rust |
| 初期成果物 | ローカルでビルド可能な`.app` |

## system-wide状態

状態取得は`/usr/bin/pmset -g`だけを実行し、system-wideの`SleepDisabled`を読む。`-g custom`へのフォールバックと電源別の`Mixed`状態は実装しない。

```text
SleepDisabled 0 -> enabled  -> 通常のmacOSスリープ動作 -> UIはOFF
SleepDisabled 非0 -> disabled -> Lid Awake有効          -> UIはON
```

成功状態は`enabled`または`disabled`のみとする。キー欠落、不正値、コマンド失敗はエラーとする。

## IPC契約

```text
SleepStatus = "enabled" | "disabled"
ChangeOutcome = "applied" | "cancelled"
get_sleep_state() -> Result<SleepStatus, String>
set_sleep_disabled(disabled: boolean) -> Result<ChangeOutcome, String>
```

Rustからフロントエンドへ`SleepStatus`を送る`sleep-state-updated`イベントと、文字列エラーを送る`sleep-state-error`イベントを公開する。

## 状態変更と認証

Rustはbooleanに応じて、以下のどちらかの固定AppleScriptだけを選ぶ。

```applescript
do shell script "/usr/bin/pmset -a disablesleep 1" with administrator privileges
do shell script "/usr/bin/pmset -a disablesleep 0" with administrator privileges
```

- `/usr/bin/osascript`を毎回新しく起動する。
- AppleScript内で認証キャンセルのerror number `-128`だけを`cancelled`へ変換する。
- それ以外の非成功はエラーにする。
- アプリはパスワードや認証情報を保持しない。
- `/bin/sh -c`、任意コマンド文字列、`sudo`を使わない。

## 同期契約

- 起動時に状態を取得する。
- Rust workerが3秒ごとに状態を取得し、イベントを送る。
- ウインドウ表示時とフォーカス時に即時取得する。
- 状態取得、周期取得、設定変更は同じ非同期mutexで直列化する。
- 設定変更中の周期取得は待たずにスキップする。
- 設定変更の成功、失敗、認証キャンセル後は必ず実状態を再取得する。
- UIは取得結果だけで更新し、クリック直後の楽観更新をしない。

## ウインドウとメニューバー

- ウインドウは固定`200 × 100px`、リサイズ不可、透明・枠なし・影付きとする。
- 34pxのドラッグ可能なタイトルバーと、176 × 44pxの単一トグルボタンだけを表示する。トグルは左右へ移動するノブとON/OFFラベルで状態を示し、クリック領域の外枠や塗りは常時表示しない。
- Dockへ表示しないAccessoryアプリとする。
- トレイ左クリックでウインドウを表示／非表示にする。
- トレイメニューから「表示」と「終了」を実行できる。
- トレイ画像は36 × 36pxの黒＋透明PNGをtemplate imageとして扱う。

## コンパクトUI

タイトルバーは5pxの状態ドットと`LID AWAKE`だけで構成する。本文は左右12px、上下13pxとする。

| 表示 | 意味／操作 |
|---|---|
| `ON` | `SleepDisabled`が非0。クリックで`disabled=false` |
| `OFF` | `SleepDisabled`が0。クリックで`disabled=true` |
| タイトルバーの`CHECKING…` | 初期取得中。ボタン内のスイッチ位置は固定 |
| タイトルバーの`UPDATING…` | 管理者認証または変更処理中。ボタン内は変更前のON/OFFを維持 |
| `RETRY`＋タイトルバーの`ERROR` | 初回取得失敗。クリックで再取得 |
| `ON/OFF`＋タイトルバーの`UPDATE FAILED` | 既知の実状態を保持した変更失敗 |
| `ON/OFF`＋タイトルバーの`SYNC FAILED` | 既知の実状態を保持した同期失敗 |

ON/OFFラベルとスイッチは固定幅で配置し、操作中に位置を動かさない。詳細エラーはbuttonのtooltipとアクセシブル説明へ保持する。認証キャンセルはエラー表示せず、再取得した実状態へ戻す。

## アイコン

- app icon masterは`1024 × 1024`、sRGB、32bit RGBA PNGとする。
- imagegenで生成した眠るナマケモノへ、均一な赤い禁止マークを決定論的に合成する。
- 外周を事前に角丸マスクしない。
- `tauri icon`で`.icns`と必要なPNGを生成する。
- tray iconは同モチーフの黒一色シルエットと禁止マークを用い、36 × 36px、黒＋透明だけにする。

## 対象外

- LaunchDaemon / LaunchAgent
- ログイン項目 / `SMAppService`
- 特権ヘルパー / `setuid` / `sudo`
- 独自の認証UI / パスワード保存
- Developer ID署名 / notarization / DMG公開 / App Store配布
- 設定を戻すアンインストーラ

`disablesleep`はmacOS側に保持されるため、アプリ削除前にOFFへ戻すことを推奨する。
