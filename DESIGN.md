# Design

## File dependency checks

Set limits in the repository-root `matis.json`:

```json
{"maxFanIn": 30, "maxFanOut": 20, "fanExclude": ["src/generated/**", "!src/generated/entry.py"]}
```

`maxFanIn` defaults to 30 and `maxFanOut` defaults to 20. Each accepts a nonnegative integer or `null` to disable that check. A finding occurs only when the count exceeds the limit. These two limits apply only to files, not modules.

Fan-in counts distinct internal files that depend on the file. Fan-out counts distinct internal files on which the file depends. Repeated edges count once. Self-dependencies and external endpoints do not count. Tests and type-only dependencies count.

`fanExclude` defaults to `[]`. It accepts an array of gitignore-style pattern strings, evaluated against repository-root-relative paths with `/` separators. A leading `/` anchors a pattern to the root; globs and `!` negations use the existing ignore matcher. Pattern order matters. For example, the configuration above excludes generated files but checks `src/generated/entry.py`.

This repository excludes only `matis-rust/src/matis/syntax.rs`, a small syntax-tree utility intentionally shared by extraction and analysis.

Exclusions suppress only fan-in and fan-out findings for matching files. They do not remove files or edges from the graph or change counts for other files. Line, complexity, change-impact, instability, circular-import, and analysis checks remain active. Existing module checks do not change. Configuration validation rejects wrong types and patterns that the ignore matcher cannot compile.

## Dashboard

A light workspace for engineers mapping unfamiliar code at their desk. Restrained color: plum-tinted neutrals with a muted plum accent for actions, selection, and graph structure. Small green dots signal an imported snapshot, not an automatically current index.

System sans typography, monospace only for CLI commands and signatures. Rounded shadcn buttons, inputs, and badges. Repository rows instead of repeated cards. Thin borders separate regions; graph nodes use soft 12px corners.

The graph workspace has full path breadcrumbs above a large canvas with a searchable right-hand contents list. Folders navigate at any depth; single-child folder chains use compact path labels. Selecting an item replaces the list with connections and source details. Conventional test files stay hidden. Outgoing dependencies outside the current scope use dashed outlines and explicit Outside scope labels. Outside-repository references and outside-scope test dependencies are excluded. Indexed boundary folders/files can be explored, and boundary functions navigate to their containing file with the function selected. On narrow screens the inspector stacks below the canvas. Motion is limited to state feedback and disabled with reduced-motion preferences.

The `dashboard/src/styles.css` entry imports tokens and base styles from `dashboard/src/styles/base.css`, graph styles from `dashboard/src/styles/graph.css`, and responsive rules from `dashboard/src/styles/responsive.css`. Graph behavior lives in `dashboard/src/components/graph/`.
