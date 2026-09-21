# 公開サイト設計 — VitePress による `site/` 再構築

公開 GitHub Pages サイトを VitePress で `site/` に完全新規構築する設計。公開/非公開の
境界規則は [PUBLICATION.md](../PUBLICATION.md)、文書の所有権と更新条件は `AGENTS.md`、
リリース手順は [RUNBOOK.md](../RUNBOOK.md) が持つ。本ファイルはサイトの構成・デプロイ・
同期義務の単一の真実。

## 背景と目的

- 旧 `docs/` の VitePress サイトは廃止済み。現在 `docs/` は contributor 文書
  （ARCHITECTURE / DESIGN / GOTCHAS）専用、`README.md` は CLI の単一の真実。
- 利用者向けの導線（導入・チャート選択・出力形式・diff・補完）が README に集中して
  いるため、VitePress サイトとして再提供する。
- `site/` に完全新規で構築し、旧サイトからコンテンツを移行しない。README や `docs/`
  の転載でもなく、サイト専用に書き下ろす。
- Pages URL は不変（`https://r-hsnin.github.io/vz/`）。GitHub リポジトリの homepage
  （`https://github.com/r-hsnin/vz`）も据え置き。

## 決定事項

### 配置とデプロイ経路

- サイトソースは `site/`、デプロイワークフローは `.github/workflows/site.yml`。
- `site.yml` は削除済みの旧 `docs.yml` を置換し、Pages デプロイを持つ唯一の
  ワークフローとなる（`docs.yml` を再導入しない）。
- base は `/vz/`（Pages URL 不変）。
- パッケージマネージャは bun。ビルド成果物は `site/.vitepress/dist`。

### コンテンツの所有

- `site/` は利用者向けコンテンツ（landing + guide）を持つ。旧 `docs/` サイトや
  README の内容をそのまま複製せず、サイト専用に構成する。
- 役割分担: `README.md` = CLI の単一の真実、`docs/` = contributor 文書、
  `site/` = 利用者向けサイト、`skills/` = エージェント向け。境界の単一の真実は
  [PUBLICATION.md §1](../PUBLICATION.md)、個別の更新条件は `AGENTS.md` の文書表。
- 同期義務: CLI・UI の挙動を変えたら `README.md` と site guide を同一コミットで
  更新する。サイトの独自記述が実装から乖離するのを防ぐ（文書表にも記載する）。

### 言語とページ構成

- en + ja の2言語。en を root、ja を `/ja/` に置く。
- landing + guide 5本: Getting Started / Chart Types / Output Modes / Diff Mode /
  Shell Completions。両言語で同一構成とする。

### 公開境界

- `site/` は `docs/` と同様の**公開専用ディレクトリ**。許可リスト
  `release-manifest.txt` に `site/` を追加する（公開範囲の変更として本設計で承認）。
- `.github/` 丸ごと許可をやめ、`.github/workflows/ci.yml` と
  `.github/workflows/site.yml` を明示列挙する。Pages 設定や今後追加され得る内部
  ワークフローを公開集合に入れない。
- `Cargo.toml` の `[package] exclude` には `site/` のみ追加する。`internal/`・
  `AGENTS.md`・`release-manifest.txt`・`lefthook.yml` は列挙しない。列挙すると
  `scripts/check-public-surface.sh` の禁止トークン検査に Cargo.toml 自身が一致して
  FAIL するため。`cargo package` は現在のリリース経路ではない（crates.io 公開は
  しない）ため、内部混入防止のための `include` allowlist 化は将来の検討事項とし、
  本設計のスコープ外とする。
- サイトコンテンツは公開面であり、[PUBLICATION.md §3](../PUBLICATION.md) の
  サニタイズ規則（P1/P2）に従う。

### 検査

- `scripts/check-public-surface.sh` は `release-manifest.txt` と git tree から
  公開集合を算出し、その集合に属するファイルだけを走査する。`site/` 追加後は
  サイト配下も検査対象になる。
- Markdown リンクは相対パスかつ `.md` 付きで書く。ルート絶対リンク（`/guide/...`）は
  公開集合外への解決として検査で落ちる。VitePress の base `/vz/` に依存する書き方を
  しない。

## サイト構成

```
site/
├── .vitepress/        # VitePress 設定・テーマ（dist/ と node_modules/ は commit しない）
├── index.md           # landing (en)
├── guide/             # en: Getting Started / Chart Types / Output Modes / Diff Mode / Shell Completions
├── ja/
│   ├── index.md       # landing (ja)
│   └── guide/         # ja: en と同一の5本
└── package.json       # bun 管理
```

## デプロイ方式

- 起動条件: `main` への push で `site/**` または `.github/workflows/site.yml` が
  変更された時、および `workflow_dispatch`。
- build ジョブ: checkout → bun セットアップ → `site/` で `bun install` →
  `bun run build` → `site/.vitepress/dist` を Pages artifact としてアップロード。
  build は dev リポジトリでも実行し、site を検証する。
- deploy ジョブ: `if: github.repository == 'r-hsnin/vz'` で public リポジトリに限定し、
  `actions/deploy-pages` で `github-pages` environment へ配信する。private の dev
  リポジトリでは deploy を走らせない。
- 権限は `contents: read` / `pages: write` / `id-token: write`。concurrency グループは
  `pages`（`cancel-in-progress: false`）。
- action の構成（checkout / setup-bun / upload-pages-artifact / deploy-pages）と
  バージョンは `site.yml` を単一の真実とする。

## スコープ外

- バージョン更新・タグ付け（v0.3.0）・本番リリースは別作業。手順は
  [RUNBOOK.md](../RUNBOOK.md)、凍結の効力と解除条件は `AGENTS.md`。
- 旧 `docs/` サイトからのコンテンツ移行。
- crates.io 公開・バイナリ配布。
- `Cargo.toml` の `include` allowlist 化による `cargo package` の内部混入防止
  （必要になった時点で再設計する）。
