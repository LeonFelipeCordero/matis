import json
import os
from pathlib import Path
import subprocess
import tempfile
import unittest

SCRIPT = Path(__file__).resolve().with_name("releaseReady.sh")
PATHS = [".github/workflows/first.yml", ".github/workflows/second.yml"]
SHA = "a" * 40
REPOSITORY = "LeonFelipeCordero/example"


def run(path, **changes):
    return {
        "path": path, "head_sha": SHA, "head_branch": "main", "event": "push",
        "head_repository": {"full_name": REPOSITORY},
        "actor": {"login": "LeonFelipeCordero"},
        "triggering_actor": {"login": "LeonFelipeCordero"},
        "run_number": 1, "run_attempt": 1, "status": "completed",
        "conclusion": "success", **changes,
    }


class ReleaseReadyTest(unittest.TestCase):
    def ready(self, runs):
        with tempfile.TemporaryDirectory() as directory:
            gh = Path(directory) / "gh"
            gh.write_text("#!/bin/sh\nprintf '%s\\n' \"$MOCK_RUNS\"\n")
            gh.chmod(0o700)
            result = subprocess.run(
                ["bash", str(SCRIPT), *PATHS], text=True, capture_output=True,
                env={**os.environ, "PATH": directory + ":" + os.environ["PATH"],
                     "GITHUB_REPOSITORY": REPOSITORY, "CI_RELEASE_SHA": SHA,
                     "MOCK_RUNS": json.dumps(runs)},
            )
        self.assertEqual(result.returncode, 0, result.stderr)
        return result.stdout.strip() == "true"

    def test_all_required_workflows_pass(self):
        self.assertTrue(self.ready([run(path) for path in PATHS]))

    def test_missing_workflow_blocks_release(self):
        self.assertFalse(self.ready([run(PATHS[0])]))
        self.assertFalse(self.ready([]))

    def test_non_success_blocks_release(self):
        for conclusion in ["failure", "cancelled", "skipped", None]:
            with self.subTest(conclusion=conclusion):
                self.assertFalse(self.ready([
                    run(PATHS[0]), run(PATHS[1], conclusion=conclusion),
                ]))

    def test_latest_attempt_must_pass(self):
        runs = [run(path) for path in PATHS]
        self.assertFalse(self.ready([
            *runs, run(PATHS[0], run_attempt=2, status="in_progress", conclusion=None),
        ]))
        self.assertTrue(self.ready([
            run(PATHS[0], conclusion="failure"), run(PATHS[1]),
            run(PATHS[0], run_attempt=2),
        ]))

    def test_other_commit_repo_branch_event_and_actor_do_not_count(self):
        for change in [
            {"head_sha": "b" * 40}, {"head_branch": "feature"},
            {"event": "pull_request"},
            {"head_repository": {"full_name": "attacker/example"}},
            {"actor": {"login": "dependabot[bot]"}},
            {"triggering_actor": {"login": "attacker"}},
        ]:
            with self.subTest(change=change):
                self.assertFalse(self.ready([run(PATHS[0]), run(PATHS[1], **change)]))


if __name__ == "__main__":
    unittest.main()
