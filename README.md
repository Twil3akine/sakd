# sakd (Sakkuri Done)

最速・便利・美しい、Rust製のCLIタスクマネージャー。

## 特徴

- **高速動作**: RustとSQLite (rusqlite) を使用した軽量設計。
- **2つのUIモード**:
  - **CLI モード**: コマンドラインから素早く操作。
  - **TUI モード**: 直感的なターミナルUIでタスクを管理（`j`/`k` で移動、`Space` で完了など）。
- **スマートな入力**:
  - 空入力で「設定なし」を設定可能。
  - 期限が既に設定されている場合はそれを初期値として表示し、未設定の場合は空から入力。
  - 時刻のデフォルトは23:59。
- **強力なショートカット**:
  - 日付: `t`, `tm`, `2d`, `1w`, `mon~sun` など。
  - 時刻: `last`, `morning`, `noon`, `1h` など。
- **洗練されたデザイン**: 
  - ミニマルなデザイン。
  - 期限の状態に応じた5段階のカラー表示。
  - ANSIカラーコードによるレイアウト崩れのない、整然としたカラム表示。
- **強力なインタラクティブモード**: 
  - `sakd` 単体で起動し、対話形式でタスクを追加・編集・完了・削除可能。
  - 項目選択や確認プロンプトによるスムーズなUX。
- **柔軟な期限管理**: 日付のみ、または時刻指定を含めた期限設定が可能。
- **自動ソート**: タスクは期限の近い順に自動的に並べ替えられます（期限なしは最後）。

## インストール

```bash
git clone https://github.com/twil3/sakd.git
cd sakd
cargo build --release
```

### パスの設定
ビルド後、生成されたバイナリ `target/release/sakd` をシステムのパスが通ったディレクトリに配置するか、エイリアスを設定することで `sakd` コマンドとして利用可能になります。

例（PowerShellの場合）:
```powershell
$env:Path += ";C:\path\to\sakd\target\release"
```

## 使い方

### モードの切り替え
- **インタラクティブモード**: 引数なしで実行すると、メニューが表示されます。
  ```bash
  sakd
  ```
- **TUIモード**: `tui` コマンドまたは `t` エイリアスで起動します。
  ```bash
  sakd tui
  # alias: t
  sakd t
  ```

### コマンドライン実行

- **タスクの追加**
  ```bash
  sakd add "タスク名"
  # alias: a
  sakd a "タスク名"
  ```

- **タスクの一覧表示**
  ```bash
  sakd list
  # alias: l
  sakd l
  # 完了済みも含める場合
  sakd l --all
  ```

- **タスクを完了にする**
  ```bash
  sakd done [ID]
  # alias: d
  sakd d [ID]
  ```

- **タスクの表示**
  ```bash
  sakd show [ID]
  # alias: s
  sakd s [ID]
  ```

- **タスクの編集**
  ```bash
  sakd edit [ID]
  # alias: e
  sakd e [ID]
  ```

- **タスクの削除**
  ```bash
  sakd remove [ID]
  # alias: r
  sakd r [ID]
  ```

## TUIモードの操作方法

- `j` / `↓`: 下に移動
- `k` / `↑`: 上に移動
- `Space` / `Enter`: タスクの完了/未完了を切り替え
- `a`: 新しいタスクを追加
- `e`: 選択中のタスクを編集
- `r`: 選択中のタスクを削除
- `h`: 完了済みタスクの表示/非表示を切り替え
- `q` / `Esc`: TUIを終了

## ライセンス

[MIT License](LICENSE)
