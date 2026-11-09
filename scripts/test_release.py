import os
import shutil
import subprocess
import tempfile
import unittest
from itertools import product
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]


class ReleaseTests(unittest.TestCase):
    def setUp(self):
        self.temporary = tempfile.TemporaryDirectory()
        self.addCleanup(self.temporary.cleanup)
        self.root = Path(self.temporary.name)
        shutil.copytree(ROOT / 'scripts', self.root / 'scripts')
        # Redirect only the test copy; production has no host configuration override.
        script = self.root / 'scripts/release.sh'
        script.write_text(script.read_text().replace('/etc/os-release', str(self.root / 'os-release')))
        (self.root / 'os-release').write_text('ID=ubuntu\n')
        (self.root / 'artifacts').mkdir()
        self.bin = self.root / 'bin'
        self.bin.mkdir()
        self.log = self.root / 'commands'
        self.env = dict(os.environ, PATH=f'{self.bin}:{os.environ["PATH"]}',
                        RELEASE_LOG=str(self.log), FIXTURE=str(self.root))
        self.mock('uname', 'echo Linux\n')
        self.mock('gh', r'''
 echo "$*" >> "$RELEASE_LOG"
 case "$*" in
   auth\ status) [ "${FAIL_AUTH:-0}" != 1 ] ;;
   *--json\ isDraft*)
     [ "${RELEASE_STATE:-missing}" != missing ] || exit 1
     echo "$RELEASE_STATE" ;;
   release\ upload*) [ "${FAIL_UPLOAD:-0}" != 1 ] ;;
   *--json\ assets*)
     printf '%s\n' "matis-1.2.3-${NATIVE_ASSET:-ubuntu.deb}"
     [ "${EXTRA_REMOTE:-0}" != 1 ] || echo unexpected.zip ;;
 esac
''')
        subprocess.run(['git', 'init', '-q', str(self.root)], check=True)

    def mock(self, name, body):
        command = self.bin / name
        command.write_text('#!/bin/bash\nset -e\n' + body)
        command.chmod(0o755)

    def run_release(self, *args, success=True):
        result = subprocess.run(['bash', 'scripts/release.sh', *args], cwd=self.root,
                                env=self.env, text=True, capture_output=True)
        self.assertEqual(result.returncode == 0, success, result.stdout + result.stderr)
        return result

    def packages(self):
        (self.root / 'artifacts/matis-1.2.3-ubuntu.deb').write_text('package')

    def commands(self):
        return self.log.read_text() if self.log.exists() else ''

    def test_draft_upload_verified_before_publish(self):
        self.packages()
        self.mock('git', 'echo abc123\n')
        self.run_release('publish', '1.2.3')
        commands = self.commands().splitlines()
        self.assertIn('--draft --target abc123', commands[1])
        self.assertTrue(commands[2].startswith('release upload v1.2.3'))
        self.assertIn('--json assets', commands[3])
        self.assertEqual(commands[4], 'release edit v1.2.3 --draft=false')

    def test_missing_source_commit_prevents_draft_creation(self):
        self.packages()
        self.run_release('publish', '1.2.3', success=False)
        self.assertNotIn('release create', self.commands())
        self.assertNotIn('release upload', self.commands())

    def test_failed_upload_or_extra_remote_assets_leave_draft(self):
        self.packages()
        self.env['RELEASE_STATE'] = 'true'
        for variable in ['FAIL_UPLOAD', 'EXTRA_REMOTE']:
            with self.subTest(variable=variable):
                self.env[variable] = '1'
                self.run_release('publish', '1.2.3', success=False)
                self.assertNotIn('release edit', self.commands())
                del self.env[variable]

    def test_published_release_is_not_modified(self):
        self.packages()
        self.env['RELEASE_STATE'] = 'false'
        self.run_release('publish', '1.2.3', success=False)
        self.assertNotIn('release upload', self.commands())

    def test_missing_empty_or_symlink_package_prevents_gh(self):
        for kind in ['missing', 'empty', 'symlink']:
            with self.subTest(kind=kind):
                package = self.root / 'artifacts/matis-1.2.3-ubuntu.deb'
                package.unlink(missing_ok=True)
                if kind == 'empty':
                    package.write_text('')
                elif kind == 'symlink':
                    package.symlink_to(self.root / 'scripts/release.sh')
                self.run_release('publish', '1.2.3', success=False)
                self.assertEqual(self.commands(), '')

    def test_other_local_platform_packages_are_not_uploaded(self):
        self.packages()
        (self.root / 'artifacts/matis-1.2.3-arch.pkg.tar.zst').write_text('other')
        self.env['RELEASE_STATE'] = 'true'
        self.run_release('publish', '1.2.3', 'ubuntu')
        upload = next(line for line in self.commands().splitlines() if line.startswith('release upload'))
        self.assertEqual(upload, 'release upload v1.2.3 artifacts/matis-1.2.3-ubuntu.deb --clobber')

    def test_invalid_arguments_and_wrong_os_do_not_build(self):
        self.mock('uname', 'echo Darwin\n')
        self.mock('cargo', 'echo cargo >> "$RELEASE_LOG"\n')
        for args in [('build', 'arch', '1.2.3'), ('build', 'windows', '1.2.3'),
                     ('build', 'macos', '01.2.3'), ('publish', 'v1.2.3'),
                     ('build',), ('version', 'extra')]:
            with self.subTest(args=args):
                self.run_release(*args, success=False)
        self.assertEqual(self.commands(), '')

    def test_version_is_read_only_and_uses_existing_selection(self):
        rust = self.root / 'matis-rust'
        rust.mkdir()
        manifest = '[package]\nname = "matis-rust"\nversion = "0.1.0"\n'
        (rust / 'Cargo.toml').write_text(manifest)
        self.mock('git', '''
if [ "$2" = --list ]; then echo v1.2.2; fi
''')
        self.assertEqual(self.run_release('version').stdout.strip(), '1.2.3')
        self.assertEqual((rust / 'Cargo.toml').read_text(), manifest)

    def prepare_build(self):
        (self.root / 'LICENSE').write_text('MIT')
        rust = self.root / 'matis-rust'
        rust.mkdir()
        (rust / 'Cargo.toml').write_text('[package]\nname = "matis-rust"\nversion = "0.1.0"\n')
        (rust / 'Cargo.lock').write_text('version = 4\n[[package]]\nname = "matis-rust"\nversion = "0.1.0"\n')
        (self.root / 'dashboard').mkdir()
        (self.root / 'dashboard/package.json').write_text('{}')
        (self.root / 'dashboard/public').mkdir()
        (self.root / 'dashboard/public/logo.png').write_bytes(b'icon')
        self.mock('cargo', r'''
echo cargo >> "$RELEASE_LOG"
[ "${FAIL_BUILD:-0}" != 1 ] || exit 1
version=$(python3 -c 'import tomllib; print(tomllib.load(open("matis-rust/Cargo.toml", "rb"))["package"]["version"])')
grep -q "version = \"$version\"" matis-rust/Cargo.lock
[ "${FAIL_BINARY:-0}" != 1 ] || version=9.9.9
mkdir -p matis-rust/target/release
printf '#!/bin/sh\necho matis %s\n' "$version" > matis-rust/target/release/matis
cp matis-rust/target/release/matis matis-rust/target/release/matis-migrate
chmod +x matis-rust/target/release/*
''')
        self.mock('bun', '''
if [ "$1 $2" = 'run build' ]; then
  mkdir -p .output/server .output/public
  echo server > .output/server/index.mjs
elif [ "$1" = build ]; then
  while [ "$1" != --outfile ]; do shift; done
  echo executable > "$2"
  chmod +x "$2"
fi
''')
        subprocess.run(['git', 'add', '.'], cwd=self.root, check=True)

    @unittest.skipUnless(shutil.which('bun') and os.uname().sysname == 'Linux', 'Requires Linux and Bun')
    def test_compiled_launcher_migrates_and_opens_loopback_dashboard(self):
        import socket
        import time
        import urllib.request
        with socket.socket() as connection:
            if connection.connect_ex(('127.0.0.1', 43127)) == 0:
                self.skipTest('Desktop port already in use')
        directory = self.root / 'usr/bin'
        directory.mkdir(parents=True)
        executable = directory / 'matis-ui'
        subprocess.run(['bun', 'build', str(ROOT / 'scripts/desktop.ts'), '--compile',
                        '--outfile', str(executable)], check=True, capture_output=True)
        migration = directory / 'matis-migrate'
        migration.write_text('#!/bin/sh\nexit 7\n')
        migration.chmod(0o755)
        result = subprocess.run([str(executable)], env=self.env, capture_output=True, timeout=10)
        self.assertNotEqual(result.returncode, 0)
        self.assertIn(b'Database migration failed', result.stderr)
        migration.write_text('#!/bin/sh\necho migrated > "$FIXTURE/migrated"\n')
        server = self.root / 'usr/lib/matis/dashboard/server'
        server.mkdir(parents=True)
        (server / 'index.mjs').write_text("Bun.serve({hostname: process.env.NITRO_HOST, port: Number(process.env.NITRO_PORT), fetch() { return new Response('dashboard') }})")
        self.mock('xdg-open', 'echo "$*" > "$FIXTURE/opened"\n')
        process = subprocess.Popen([str(executable)], env=self.env, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
        try:
            for attempt in range(100):
                if (self.root / 'opened').exists():
                    break
                time.sleep(0.05)
            self.assertEqual((self.root / 'migrated').read_text().strip(), 'migrated')
            self.assertEqual((self.root / 'opened').read_text().strip(), 'http://127.0.0.1:43127')
            with urllib.request.urlopen('http://127.0.0.1:43127') as response:
                self.assertEqual(response.read(), b'dashboard')
        finally:
            process.terminate()
            process.wait(timeout=10)

    def prepare_native_release(self, platform='ubuntu'):
        self.prepare_build()
        (self.root / 'os-release').write_text(f'ID={platform}\n')
        self.env['NATIVE_ASSET'] = 'ubuntu.deb' if platform == 'ubuntu' else 'arch.pkg.tar.zst'
        self.mock('uname', '[ "$1" = -m ] && echo x86_64 || echo Linux\n')
        self.mock('makepkg', r"""
cp -a root "$FIXTURE/payload"
version=$(sed -n 's/^pkgver=//p' PKGBUILD)
root/usr/bin/matis --version > "matis-$version-1-x86_64.pkg.tar.zst"
cp PKGBUILD "$FIXTURE/metadata"
""")
        self.mock('dpkg', 'echo amd64\n')
        self.mock('dpkg-deb', r"""
cp -a "$3" "$FIXTURE/payload"
"$3/usr/bin/matis" --version > "$4"
cp "$3/DEBIAN/control" "$FIXTURE/metadata"
""")
        subprocess.run(['git', '-c', 'user.name=Test', '-c', 'user.email=test@example.com',
                        'commit', '-qm', 'fixture'], cwd=self.root, check=True)
        real_git = shutil.which('git')
        self.mock('git', f"""
if [ "$1" = fetch ]; then
  echo "$*" >> "$RELEASE_LOG"
  [ "${{FAIL_FETCH:-0}}" != 1 ] || exit 1
  "{real_git}" tag v1.2.2
  # A fetched stable tag must not point at HEAD.
  "{real_git}" -c user.name=Test -c user.email=test@example.com commit --allow-empty -qm next
else
  exec "{real_git}" "$@"
fi
""")

    def test_no_arguments_ubuntu_release(self):
        self.assert_native_release('ubuntu')

    def test_no_arguments_arch_release(self):
        self.assert_native_release('arch')

    def assert_native_release(self, platform):
        self.prepare_native_release(platform)
        self.run_release()
        payload = self.root / 'payload/usr'
        for name in ['bin/matis', 'bin/matis-migrate', 'bin/matis-ui',
                     'lib/matis/dashboard/server/index.mjs', 'share/applications/matis.desktop']:
            self.assertTrue((payload / name).is_file(), name)
        self.assertIn('Exec=matis-ui', (payload / 'share/applications/matis.desktop').read_text())
        self.assertIn('1.2.3', (self.root / 'metadata').read_text())
        artifacts = list((self.root / 'artifacts').iterdir())
        self.assertEqual([file.name for file in artifacts], [f'matis-1.2.3-{self.env["NATIVE_ASSET"]}'])
        self.assertEqual(artifacts[0].read_text().strip(), 'matis 1.2.3')
        self.assertIn('0.1.0', (self.root / 'matis-rust/Cargo.toml').read_text())
        self.assertIn('0.1.0', (self.root / 'matis-rust/Cargo.lock').read_text())
        commands = self.commands().splitlines()
        self.assertEqual(commands[:3], ['auth status', 'fetch origin --tags', 'cargo'])
        self.assertEqual(sum(line.startswith('release upload') for line in commands), 1)
        self.assertEqual(commands[-1], 'release edit v1.2.3 --draft=false')
        self.assertFalse(any(word in self.commands() for word in ['macos', 'docker', 'podman', 'ssh']))

    def test_no_arguments_short_circuits_failed_boundaries(self):
        self.prepare_native_release()
        for variable, forbidden in [('FAIL_AUTH', 'fetch'), ('FAIL_FETCH', 'cargo'),
                                    ('FAIL_BUILD', 'release'), ('FAIL_BINARY', 'release'),
                                    ('FAIL_UPLOAD', 'release edit'),
                                    ('EXTRA_REMOTE', 'release edit')]:
            with self.subTest(variable=variable):
                self.log.unlink(missing_ok=True)
                self.env[variable] = '1'
                self.run_release(success=False)
                self.assertNotIn(forbidden, self.commands())
                del self.env[variable]
                subprocess.run([shutil.which('git'), 'tag', '-d', 'v1.2.2'],
                               cwd=self.root, capture_output=True, check=False)

    def test_invalid_selected_version_prevents_build_and_publish(self):
        self.prepare_native_release()
        self.mock('python3', 'echo 01.2.3\n')
        self.run_release(success=False)
        self.assertEqual(self.commands().splitlines(), ['auth status', 'fetch origin --tags'])

    def test_unsupported_hosts_fail_before_side_effects(self):
        hosts = [('Darwin', 'ID=arch'), ('Linux', 'ID=fedora'), ('Linux', 'ID=debian')]
        commands = [(), ('build', 'ubuntu', '1.2.3'), ('publish', '1.2.3')]
        for (host, identity), args in product(hosts, commands):
            with self.subTest(host=host, identity=identity, args=args):
                self.mock('uname', f'echo {host}\n')
                (self.root / 'os-release').write_text(identity + '\n')
                self.run_release(*args, success=False)
                self.assertEqual(self.commands(), '')
                self.assertEqual(list((self.root / 'artifacts').iterdir()), [])

    def test_id_like_and_native_id_detection(self):
        self.packages()
        self.env['RELEASE_STATE'] = 'true'
        for identity in ['ID=derivative\nID_LIKE="debian ubuntu"\n',
                         'ID=ubuntu\nID_LIKE=arch\n']:
            with self.subTest(identity=identity):
                (self.root / 'os-release').write_text(identity)
                self.run_release('publish', '1.2.3')
        self.run_release('publish', '1.2.3', 'arch', success=False)


if __name__ == '__main__':
    unittest.main()
