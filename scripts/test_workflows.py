import re
import unittest
from pathlib import Path
from types import SimpleNamespace


ROOT = Path(__file__).resolve().parents[1]
WORKFLOWS = ROOT / '.github/workflows'


def condition(workflow, job):
    block = workflow.split(f'  {job}:\n', 1)[1]
    return re.search(r'    if: >-\n((?:      .+\n)+)', block)[1].strip()


def allowed(expression, **context):
    expression = expression.replace('&&', ' and ').replace('||', ' or ')
    return bool(eval(expression.replace('\n', ' '), {'__builtins__': {}},
                     {'github': SimpleNamespace(**context)}))


class WorkflowTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.trusted = (WORKFLOWS / 'trustedCi.yml').read_text()
        cls.release = (WORKFLOWS / 'release.yml').read_text()

    def trusted_context(self, event='push'):
        repo = SimpleNamespace(full_name='LeonFelipeCordero/matis')
        pr = SimpleNamespace(head=SimpleNamespace(repo=repo),
                             base=SimpleNamespace(repo=repo, ref='main'), state='open')
        return dict(repository=repo.full_name, actor='LeonFelipeCordero', actor_id='1',
                    repository_owner='LeonFelipeCordero', repository_owner_id='1',
                    triggering_actor='LeonFelipeCordero', event_name=event,
                    ref='refs/heads/main', event=SimpleNamespace(pull_request=pr))

    def release_context(self):
        run = SimpleNamespace(actor=SimpleNamespace(login='LeonFelipeCordero', id='1'),
                              triggering_actor=SimpleNamespace(login='LeonFelipeCordero'),
                              conclusion='success', event='push', head_branch='main',
                              head_repository=SimpleNamespace(full_name='LeonFelipeCordero/matis'),
                              path='.github/workflows/trustedCi.yml')
        return dict(repository='LeonFelipeCordero/matis', repository_owner='LeonFelipeCordero',
                    repository_owner_id='1', event=SimpleNamespace(workflow_run=run))

    def test_only_owner_main_push_or_same_repo_main_pr_is_trusted(self):
        expression = condition(self.trusted, 'validate')
        for event in ['push', 'pull_request']:
            with self.subTest(event=event):
                self.assertTrue(allowed(expression, **self.trusted_context(event)))
        for field, value in [('repository', 'other/matis'), ('actor', 'other'),
                             ('actor_id', '2'), ('triggering_actor', 'other'),
                             ('ref', 'refs/heads/other'), ('event_name', 'workflow_dispatch')]:
            with self.subTest(field=field):
                context = self.trusted_context()
                context[field] = value
                self.assertFalse(allowed(expression, **context))
        for boundary in ['head', 'base', 'target', 'closed']:
            with self.subTest(boundary=boundary):
                context = self.trusted_context('pull_request')
                pr = context['event'].pull_request
                if boundary in ['head', 'base']:
                    getattr(pr, boundary).repo = SimpleNamespace(full_name='fork/matis')
                elif boundary == 'target':
                    pr.base.ref = 'other'
                else:
                    pr.state = 'closed'
                self.assertFalse(allowed(expression, **context))

    def test_release_requires_successful_canonical_main_push_and_workflow_path(self):
        expression = condition(self.release, 'release')
        self.assertTrue(allowed(expression, **self.release_context()))
        cases = [('conclusion', 'failure'), ('conclusion', 'cancelled'),
                 ('conclusion', 'skipped'), ('conclusion', 'timed_out'),
                 ('event', 'pull_request'), ('event', 'workflow_dispatch'),
                 ('head_branch', 'other'), ('path', '.github/workflows/other.yml'),
                 ('head_repository', SimpleNamespace(full_name='fork/matis')),
                 ('actor', SimpleNamespace(login='other', id='1')),
                 ('actor', SimpleNamespace(login='LeonFelipeCordero', id='2')),
                 ('triggering_actor', SimpleNamespace(login='other'))]
        for field, value in cases:
            with self.subTest(field=field, value=value):
                context = self.release_context()
                setattr(context['event'].workflow_run, field, value)
                self.assertFalse(allowed(expression, **context))
        context = self.release_context()
        context['repository'] = 'fork/matis'
        self.assertFalse(allowed(expression, **context))

    def test_trusted_contract_and_all_checks_remain(self):
        self.assertTrue(self.trusted.startswith('name: Trusted CI\n'))
        self.assertEqual(re.findall(r'^  (\w+):$', self.trusted.split('jobs:\n')[1], re.M),
                         ['validate', 'checks'])
        self.assertEqual(re.findall(r'^    name: (.+)$', self.trusted, re.M),
                         ['Validate trusted event', 'Trusted checks'])
        self.assertIn('  pull_request:\n    branches: [main]\n', self.trusted)
        self.assertIn('types: [opened, synchronize, reopened, ready_for_review]', self.trusted)
        self.assertIn('permissions:\n  contents: read\n', self.trusted)
        self.assertIn('needs: validate', self.trusted)
        self.assertIn('runs-on: matato-ci-${{ github.run_id }}-${{ github.run_attempt }}', self.trusted)
        self.assertIn('ref: ${{ github.sha }}\n          persist-credentials: false', self.trusted)
        self.assertNotIn('environment:', self.trusted)
        self.assertNotIn('GH_TOKEN', self.trusted)
        self.assertNotIn('frozen-lockfile', self.trusted)
        for command in ["python3 -m unittest discover -s scripts -p 'test_*.py'",
                        'bash -n scripts/release.sh',
                        'cargo fmt --manifest-path matis-rust/Cargo.toml --check',
                        'cargo test --manifest-path matis-rust/Cargo.toml --locked --all-targets',
                        'cargo clippy --manifest-path matis-rust/Cargo.toml --locked --all-targets -- -D warnings',
                        'cargo build --manifest-path matis-rust/Cargo.toml --release --locked --bins',
                        'matis-rust/target/release/matis check --f . --format json',
                        'bun install --no-save', 'bun run build', 'bun test',
                        'bun run typecheck', 'bun run lint']:
            self.assertIn(command, self.trusted)
        self.assertNotIn('--fix', self.trusted)

    def test_hosted_release_uses_exact_source_and_scoped_token(self):
        for text in ['workflows: [Trusted CI]', 'types: [completed]', 'branches: [main]',
                     'permissions: {}', 'cancel-in-progress: false', 'queue: max', 'runs-on: ubuntu-24.04',
                     '    permissions:\n      contents: write',
                     'ref: ${{ github.event.workflow_run.head_sha }}', 'fetch-depth: 0',
                     'persist-credentials: false', 'bash scripts/release.sh version',
                     'bash scripts/release.sh status "$version"',
                     'bash scripts/release.sh build ubuntu "$VERSION"',
                     'bash scripts/release.sh publish "$VERSION" ubuntu']:
            self.assertIn(text, self.release)
        self.assertEqual(self.release.count("if: steps.version.outputs.state != 'published'"), 4)
        self.assertEqual(self.release.count('GH_TOKEN:'), 2)
        self.assertNotIn('continue-on-error', self.release)
        self.assertNotIn('|| true', self.release)
        for action in re.findall(r'uses: (.+)', self.trusted + self.release):
            self.assertRegex(action, r'^[\w/-]+@[0-9a-f]{40}(?: #.*)?$')


if __name__ == '__main__':
    unittest.main()
