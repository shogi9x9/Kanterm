# MCP でカードを別ボードへ移動する

## 概要

`update_card` は `move_to_board` を受け取り、指定したカードを別ボードへ移動できます。
既定の `backlog` ボードは予約済みの inbox / planning 用ボードで、列は `Backlog`
1 本だけです。作業対象が決まったカードは、プロジェクト用ボードへ移動します。

- `key` には移動対象カードのキー（例: `KB-1`）を指定。
- `move_to_board` には、移動先ボードの **slug**（例: `work`）を指定。
- 任意で `column` を指定すると、移動先ボード内の指定列へ同時に移動。

## 例

```json
{
  "tool": "update_card",
  "arguments": {
    "key": "KB-1",
    "move_to_board": "work",
    "column": "This week"
  }
}
```

## 仕様上の注意

- `move_to_board` で移動すると、キーは移動先ボードの採番体系に従って更新されるため、
  返却キーは変わる場合があります（例: `KB-1` → `WB-1`）。
- 既存列を指定しない場合は、移動先ボードの先頭列へ移動します。
- Backlog ボードには `Backlog` 列しかありません。プロジェクト用ボードの列は
  作成時に選んだテンプレートで変わるため、`column` を指定する前に `get_board`
  で移動先の列を確認してください。
- 先に `get_board` で `boards:` 行を確認すると、利用可能なボード名が分かります。

## エラー時の代表例

- 未知の board slug を指定した場合: エラーになります。
- 存在しない列名を `column` として指定した場合: エラーになります。

### 関連 CLI/MCP の操作

- 対象ボードの確認: `get_board` / `list_cards`
- 移動先プロジェクトボード作成: `manage_boards` (`action: create`, `template`)
- ボード横断操作（MCP）: まず `get_board` で current board を確認し、`move_to_board` を指定
