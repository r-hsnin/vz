# 公開範囲とリリース設計

この文書は「何を公開し、何を内部に留めるか」と、その境界を守る仕組みの単一の真実。
公開許可リストそのものは `release-manifest.txt`、リリース手順・復旧は
[RUNBOOK.md](RUNBOOK.md)、文書の所有権表は `AGENTS.md` が持つ。ここには境界の意図と
規則を置き、手順の逐条は重複させない。

リリース先は **公開 GitHub リポジトリ（`origin`）のみ**。crates.io・バイナリ配布は
対象外（将来の検討事項は §8）。利用者向けサイトは `site/` から GitHub Pages
（`https://r-hsnin.github.io/vz/`）へ配信する。設計は
[specs/2026-09-21-public-site-design.md](specs/2026-09-21-public-site-design.md) が持つ。

> **凍結中:** 現在 `origin` へのリリースは行わない。`scripts/release.sh` は実行しない。
> この文書の存在をリリースが有効である根拠としない。日常作業は `dev` への push のみ。

## 1. 公開/非公開の境界

`release-manifest.txt` は許可リストであり、**記載の無いパスは非公開が既定**。境界は
「利用者・貢献者に必要なものは公開、維持運用に固有のものは内部」で引く。

| 公開（manifest 記載） | 非公開（manifest 非記載） |
|---|---|
| `README.md` / `LICENSE` / `CONTRIBUTING.md` | `AGENTS.md`（root 維持） |
| `Cargo.toml` / `Cargo.lock` / `rust-toolchain.toml` | `release-manifest.txt` / `lefthook.yml` / `scripts/` |
| `.github/workflows/ci.yml` / `.github/workflows/site.yml` / `.gitignore` | `internal/` |
| `src/` / `tests/` / `benches/` / `fixtures/` / `demo/` | `target/`（git 管理外） |
| `docs/`（公開専用）/ `site/`（公開専用）/ `skills/` | |

- `docs/` は丸ごと許可のまま**公開専用ディレクトリ**とする。内部文書は `internal/` に置く。
- `internal/` は default-private のため manifest への追記は不要（追記しないこと）。
- `rust-toolchain.toml` は公開する。toolchain 固定は公開コードの再現性に資し、
  `CONTRIBUTING.md` が前提としている。
- `site/` も**公開専用ディレクトリ**とし、利用者向けの VitePress サイト（landing +
  guide 5本、en/ja）を置く。旧 `docs/` サイトからのコンテンツ移行は行わない。
- `.github/` は丸ごと許可とせず、`workflows/ci.yml` と `workflows/site.yml` を明示列挙
  する。Pages 設定や今後追加され得る内部ワークフローを公開集合に入れないため。

公開面の役割分担: `README.md` = CLI の単一の真実、`docs/` = contributor 文書、
`site/` = 利用者向けサイト、`skills/` = エージェント向け。個別文書の所有権と更新条件は
`AGENTS.md` の文書表が持つ。

### なぜ `AGENTS.md` は root に残すか

`AGENTS.md` は AI エージェントがリポジトリ root から自動読込する運用ガイドであり、
`internal/` へ移すと読込が働かなくなる。非公開のまま root に置く限り漏洩は生じない
ため、移動しない。所有権表のパスだけを実体に合わせて更新する。

## 2. manifest の意味論

- ディレクトリは末尾 `/`。配下すべてを許可する。
- ファイルは完全一致。
- 空行と `#` 始まりは無視。
- **manifest の変更は公開範囲の変更**であり、明示的な承認を要する。黙って広げない。

## 3. 公開ドキュメントのサニタイズ規則

公開docは利用者と外部貢献者に向ける。次を守る。

- **P1 非公開参照の禁止。** 公開ファイルに非公開パス・非公開リポジトリ名・remote 運用・
  hook・`scripts/` を書かない。相対リンクは公開集合内で解決すること。
- **P2 内部プロセスの禁止。** 凍結状態・リリース手順・公開範囲・ロードマップ
  （`Status:` 等）・残負債一覧は公開docに書かない。`internal/` に置く。
- **P3 設計意図は公開する。** why・却下した代替・意図的な乖離・利用者が踏む既知のバグは
  公開のまま残す。隠すのではなく、内部プロセスと混ぜないことが目的。

禁止トークン（検査ツールが公開予定ファイルに対して走査する。追加時は
`scripts/check-public-surface.sh` の一覧も更新する）:

```
AGENTS   release-manifest   r-hsnin/vz-dev   vz-dev   dev/main
guard-origin   lefthook   scripts/hooks   scripts/release.sh
RUNBOOK   PLAN.md   internal/
origin/main   origin remote
```

`rust-toolchain.toml`・`r-hsnin/vz`（公開リポジトリ）・`docs/` 配下の公開doc名は許可。

## 4. 検査ツール `scripts/check-public-surface.sh`

非公開スクリプト（`scripts/` は manifest 非記載）。manifest と git tree から公開予定
集合を算出し、**その集合に属するファイルだけ**を検査する。内部ファイルが内部物を
参照するのは正常であり、対象外とする。

検査項目:

1. 公開予定ファイルに禁止トークンが無いこと（`file:line` を報告）。
2. 公開docの相対 Markdown リンクが公開集合内で解決すること。
3. manifest の全パターンが実ファイルに 1 件以上一致すること（drift 検知）。

- 使い方: `./scripts/check-public-surface.sh [--tag <ref>]`。`--tag` 省略時は作業ツリー、
  指定時はその tag の tree を対象にする。
- 違反があれば非ゼロ終了。CI ではなくローカルで完結させる（非公開スクリプトのため
  外部貢献者には見せない）。
- 組込み先: `lefthook.yml` の pre-commit、および `scripts/release.sh` の分岐前。
  `CONTRIBUTING.md` には載せない（非公開パスを公開面に出さないため）。

## 5. 公開前チェック（凍結解除後）

1. `./scripts/check-public-surface.sh` が PASS。
2. `./scripts/release.sh <tag> --dry-run` の Allowed/Excluded を目視。
3. `Cargo.toml` の version と tag の一致。
4. 上記のうえで本実行し、PR をレビュー。

## 6. 今回の変更インベントリ

- 移動: `docs/RUNBOOK.md` → `internal/RUNBOOK.md`。
- 新規: `internal/PUBLICATION.md`（本ファイル）、`internal/ROADMAP.md`（公開docから抜く
  内部ロードマップ・Status・負債・凍結メモの受け皿）、`scripts/check-public-surface.sh`。
- manifest: `rust-toolchain.toml` を追加。`internal/` は追加しない。
- 公開doc修正: `README.md` / `CONTRIBUTING.md` / `docs/ARCHITECTURE.md` /
  `docs/DESIGN.md` / `docs/GOTCHAS.md` を §3 の規則で是正。
- `AGENTS.md`: 所有権表のパス更新（`RUNBOOK.md` → `internal/RUNBOOK.md`、`internal/` 追記）。
- `Cargo.toml`: `exclude` から `release-manifest.txt` を除去（非公開名の露出解消）。
- `lefthook.yml` / `scripts/release.sh`: 検査ツールを組込み。

## 7. 検証

- `check-public-surface.sh` が PASS。
- 公開予定集合の相対リンクがすべて解決。
- `cargo fmt` / `cargo clippy --all-targets -- -D warnings` / `cargo test` に影響なし。
- 公開docに禁止トークンが 0 件。

## 8. 将来の検討（未決）

- crates.io 公開・バイナリ配布は対象外。必要になった時点で `Cargo.toml` の metadata と
  `exclude` を公開契約として再設計する。
- `Cargo.toml` の `exclude` は現状 crates.io 非公開のため実質 `cargo package` 専用。
  `site/` のみを追加する。`internal/`・`AGENTS.md`・`release-manifest.txt`・
  `lefthook.yml` を列挙すると `scripts/check-public-surface.sh` の禁止トークン検査に
  Cargo.toml 自身が一致して FAIL するため、内部混入防止の抜本化は `include` allowlist
  化を含め将来の検討事項とする。
- L3（`vz-core` / `vz` 二分割）実施時は manifest を `src/` から `crates/*/src` へ変更する
  必要がある（公開範囲の変更として承認を要する）。計画は `internal/ROADMAP.md`。
