#!/usr/bin/env bash
set -euo pipefail

: "${GITHUB_REPOSITORY:?}"
: "${CI_RELEASE_SHA:?}"
[[ $# -gt 0 ]]

gh api --method GET "repos/$GITHUB_REPOSITORY/actions/runs" \
  -f head_sha="$CI_RELEASE_SHA" -f event=push -f per_page=100 \
  --paginate --jq '.workflow_runs' |
  jq --slurp --arg sha "$CI_RELEASE_SHA" --arg repo "$GITHUB_REPOSITORY" --args '
    add as $runs |
    all($ARGS.positional[];
      . as $path |
      [$runs[] | select(.path == $path and .head_sha == $sha and
        .head_branch == "main" and .event == "push" and
        .head_repository.full_name == $repo and
        .actor.login == ($repo | split("/")[0]) and
        .triggering_actor.login == ($repo | split("/")[0]))] |
      max_by([.run_number, .run_attempt]) |
      .status == "completed" and .conclusion == "success")
  ' "$@"
