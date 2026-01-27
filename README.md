# sakd (Sakkuri Done)

最速・便利・美しい、Rust製のCLIタスクマネージャー。

## 特徴

- **高速動作**: RustとSQLite (rusqlite) を使用した軽量設計。
- **スマートな入力**:
  - 空入力で「設定なし」、Enterでデフォルト値（時刻なら23:59）を設定。
- **強力なショートカット**:
  - 日付: `t`, `tm`, `2d`, `1w`, `mon~sun` など。
  - 時刻: `last`, `morning`, `noon`, `1h` など。
- **洗練されたUI**: 
  - ミニマルなリスト表示。
  - 期限の状態に応じた5段階のカラー表示。
  - ANSIカラーコードによるレイアウト崩れのない、整然としたカラム表示。
- **強力なインタラクティブモード**: 
  - `sakd` 単体で起動し、対話形式でタスクを追加・編集・完了・削除可能。
  - 項目選択や確認プロンプトによるスムーズなUX。
- **柔軟な期限管理**: 日付のみ、または時刻指定を含めた期限設定が可能。

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

### インタラクティブモード
引数なしで実行すると、メニューが表示されます。
```bash
sakd
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

## ライセンス

[MIT License](LICENSE)
