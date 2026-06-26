# TUI リファレンス

English version: [tui.md](tui.md)

```sh
./target/release/kanterm
```

## ボードのキー操作

| キー           | 操作                                |
| -------------- | ----------------------------------- |
| `h` / `l`      | 列間のフォーカス移動                |
| `j` / `k`      | 列内の選択移動                      |
| `H` / `L`      | 選択中カードを左右へ移動            |
| `J` / `K`      | カードを列内で下/上へ並べ替え       |
| `n`            | 新規カード（フォーカス中の列に）    |
| `N`            | 新規ボード                          |
| `b`            | 一覧からボードを選択（`J/K` でボード並べ替え） |
| `i`            | 現在ボードの agent context を編集（空入力でクリア） |
| `M`            | 選択カードを別ボード/列へ移動（Backlog 上では `send-project`） |
| `c`            | 列管理（追加/変更/並べ替え/削除）   |
| `Tab`          | 次のボードへ切替                    |
| `w`            | 次のローカル作業候補へジャンプ      |
| `/`            | カードを絞り込み（title/body/label）|
| `Enter`        | カードの **詳細モーダル** を開く    |
| `e`            | 選択カードのタイトルを簡易編集      |
| `p`            | 優先度を循環（`[L]` → `[M]` → `[H]`）|
| `u`            | 直近の取り消し可能なカード更新を戻す |
| `d`            | 選択カードをアーカイブ              |
| `m`            | memory browser（意思決定/学習）     |
| `D`            | 現在のボードをアーカイブ            |
| `U`            | アーカイブ済みボード: `Enter` 復元、`d` 削除 |
| `r`            | ディスクから再読込                  |
| `q` / `Esc`    | 終了                                |

`/` を押すと、各列を title / body / label に一致するカードへ絞り込むフィルタが
開きます。空のフィルタを送信するとクリアされます。

## 詳細モーダル

**詳細モーダル** では: `e` タイトル · `b` 本文編集（複数行）· `M` 別ボード/列へ
移動 · `p` 優先度 · `a` 担当者 · `D` 期日（`YYYY-MM-DD`、空でクリア）· `t`
ラベル · `x` 任意メモ付きで完了 · `d` アーカイブ · `Esc` 戻る。
完了すると、カードはアーカイブされ、`agent_state=done` になり、進行中の handoff
field と claim がクリアされます。期限切れカードは赤い `⏰` チップを表示します。

ボード画面で `u` を押すと、アクティブボード上の直近の取り消し可能なカード更新
（誤アーカイブ、完了、カード移動など）を戻せます。完全削除は意図的に undo 対象外
です。

**本文エディタ** では: 矢印で移動、`Enter` 改行、`Ctrl-S` 保存、`Esc` キャンセル。

## label picker

**label picker**（`t`）はポップアップです。名前を入力して `Enter` で新しいラベル
を追加するか、`↑/↓` と `Space` で既出ラベルの ON/OFF を切り替えます（いくつでも
付与可）。一覧を整理するため、直近 1 か月以内に使われたラベルのみ候補に出ます
（カードに付いているラベルは常に切り替え可能）。

## ボード

既定の **Backlog** ボード（slug: `backlog`）は予約済みの inbox / planning 用
ボードです。このボードには **Backlog** 列 1 本だけが存在し、列の追加/変更/削除/
並べ替えはできません。また `Backlog` という名前のボードを追加で作成することも
できません。

プロジェクト用に新規作成するボードは、開発者向けの `workflow`
（**Todo · In progress · Testing · Waiting for release**）が既定です。必要に応じて
`planning`（**Backlog · Today · This week · This month**）や `simple`
（**Todo · Doing · Done**）も選べます。列は `c` で管理します（追加・変更・並べ替え
`J/K`・削除）。列削除時は、その列のカードをどの列へ移すか尋ねられます。

フォーカス中の列・選択中のカード・**アクティブボード** は起動間で記憶されます。
ヘッダ行はアクティブボードをコンパクトなセレクタ（`board < name > 1/N`）で表示し、
`Tab` でボードを循環、`b` でボード一覧を開けます。

完了したボードは削除ではなく **アーカイブ**（`D`）します。タブからは消えますが、
カードはすべて保持されます。`U` でアーカイブ済みボードを一覧し、復元（`Enter`）か
完全削除（`d` の後に `delete` と入力）ができます。削除は先にアーカイブが必要で、
Backlog ボードはどちらもできません。

## テーマ

組み込みの `dark` / `light` テーマがあります:

```sh
KANBAN_THEME=light ./target/release/kanterm
```

JSON ファイルと `KANBAN_THEME_FILE` で主要色を上書きできます:

```json
{
  "accent": "cyan",
  "warning": "yellow",
  "danger": "red",
  "success": "green",
  "priority_high": "#ff5555",
  "priority_normal": "yellow",
  "priority_low": "blue"
}
```

指定できる値は `red`・`light_cyan`・`dark_gray` のような ANSI カラー名か、
`#ff5555` のような hex カラーです。

## memory log

ボードとは別に **memory log** があります。意思決定・学習・文脈など、エージェント
のセッションを跨いで残すべき情報を、グローバルな key（`M-1`、`M-2`, …）で管理し、
任意でカードに紐付けます。エージェントは MCP ツール（`record_memory` /
`recall_memories`）で読み書きし、TUI では `m` で読み取り専用ブラウザを開けます
（`Enter` 詳細、`d` アーカイブ）。memory はカードに key テキストでのみ紐付くため、
ボードのアーカイブや削除より長く残ります。エージェントの recall は TUI 閲覧とは
別に記録され、月に一度、エージェント recall のない 30 日より古い memory が消去
されます。

## エクスポート

```sh
./target/release/kanterm --export md     # Markdown（git 向き）
./target/release/kanterm --export json   # 完全な JSON スナップショット
```

## バックアップ / 復元

復元用途では SQLite DB レベルのバックアップを使います。`--export` はレビューし
やすいスナップショット出力であり、完全な import 用ではありません。

```sh
./target/release/kanterm --backup-db ./kanban-backup.db
./target/release/kanterm --restore-db ./kanban-backup.db --force
```

バックアップは SQLite の `VACUUM INTO` で作成するため、WAL に残っている内容も
一貫した形で含まれます。復元時は入力 DB の schema version を検証し、既存 DB の
置き換えには `--force` を必須にします。より新しい kanban で作られたバックアップ
は、古いバイナリでは復元を拒否します。新しいローカルリリースバイナリを実際の
ボードで試す前に、リポジトリ外へタイムスタンプ付きのバックアップを取ってください。
例:

```sh
./target/release/kanterm --backup-db ~/kanban-backups/kanban-$(date +%Y%m%d-%H%M%S).db
```
