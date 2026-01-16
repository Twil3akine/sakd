# td (Task Done)

最速・便利・美しい、Rust製のCLIタスクマネージャー。

## 特徴

- **高速動作**: RustとSQLite (rusqlite) を使用した軽量設計。
- **洗練されたUI**: 
  - ミニマルなリスト表示。
  - 期限の状態に応じた5段階のカラー表示。
  - ANSIカラーコードによるレイアウト崩れのない、整然としたカラム表示。
- **強力なインタラクティブモード**: 
  - `td` 単体で起動し、対話形式でタスクを追加・編集・完了・削除可能。
  - 項目選択や確認プロンプトによるスムーズなUX。
- **柔軟な期限管理**: 日付のみ、または時刻指定を含めた期限設定が可能。

## インストール

```bash
git clone <repository-url>
cd td
cargo build --release
```

### パスの設定
ビルド後、生成されたバイナリ `target/release/td` をシステムのパスが通ったディレクトリに配置するか、エイリアスを設定することで `td` コマンドとして利用可能になります。

例（PowerShellの場合）:
```powershell
$env:Path += ";C:\path\to\td\target\release"
```

## 使い方

### インタラクティブモード
引数なしで実行すると、メニューが表示されます。
```bash
td
```

### コマンドライン実行

- **タスクの追加**
  ```bash
  td add "タスク名"
  # alias: a
  td a "タスク名"
  ```

- **タスクの一覧表示**
  ```bash
  td list
  # alias: l
  td l
  # 完了済みも含める場合
  td l --all
  ```

- **タスクを完了にする**
  ```bash
  td done [ID]
  # alias: d
  td d [ID]
  ```

- **タスクの表示**
  ```bash
  td show [ID]
  # alias: s
  td s [ID]
  ```

- **タスクの編集**
  ```bash
  td edit [ID]
  # alias: e
  td e [ID]
  ```

- **タスクの削除**
  ```bash
  td remove [ID]
  # alias: r
  td rm [ID]
  ```

## ライセンス

[MIT License](LICENSE)
