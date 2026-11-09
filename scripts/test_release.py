import os
import shutil
import subprocess
import tempfile
import unittest
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]


class ReleaseTests(unittest.TestCase):
    def setUp(self):
        self.temporary = tempfile.TemporaryDirectory()
        self.addCleanup(self.temporary.cleanup)
        self.root = Path(self.temporary.name)
        shutil.copytree(ROOT / 'scripts', self.root / 'scripts')
        (self.root / 'artifacts').mkdir()
        self.bin = self.root / 'bin'
        self.bin.mkdir()
        self.log = self.root / 'commands'
        self.env = dict(os.environ, PATH=f'{self.bin}:{os.environ["PATH"]}',
                        RELEASE_LOG=str(self.log), FIXTURE=str(self.root))
        self.mock('gh', r'''
 echo "$*" >> "$RELEASE_LOG"
 case "$*" in
   *--json\ isDraft*)
     [ "${RELEASE_STATE:-missing}" != missing ] || exit 1
     echo "$RELEASE_STATE" ;;
   release\ upload*) [ "${FAIL_UPLOAD:-0}" != 1 ] ;;
   *--json\ assets*)
     printf '%s\n' matis-1.2.3-ubuntu.deb matis-1.2.3-arch.pkg.tar.zst matis-1.2.3-macos.zip
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
        for suffix in ['ubuntu.deb', 'arch.pkg.tar.zst', 'macos.zip']:
            (self.root / 'artifacts' / f'matis-1.2.3-{suffix}').write_text('package')

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

    def test_missing_empty_symlink_or_extra_local_assets_prevent_gh(self):
        for kind in ['missing', 'empty', 'symlink', 'extra']:
            with self.subTest(kind=kind):
                self.packages()
                package = self.root / 'artifacts/matis-1.2.3-macos.zip'
                if kind == 'missing':
                    package.unlink()
                elif kind == 'empty':
                    package.write_text('')
                elif kind == 'symlink':
                    package.unlink()
                    package.symlink_to(self.root / 'artifacts/matis-1.2.3-ubuntu.deb')
                else:
                    package.unlink(missing_ok=True)
                    self.packages()
                    (self.root / 'artifacts/matis-1.2.3-extra.zip').write_text('extra')
                self.run_release('publish', '1.2.3', success=False)
                self.assertEqual(self.commands(), '')
                package.unlink(missing_ok=True)

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
        self.mock('cargo', '''
mkdir -p matis-rust/target/release
printf '#!/bin/sh\necho matis 1.2.3\n' > matis-rust/target/release/matis
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

    def test_native_mac_bundle_with_mocked_build_tools(self):
        self.prepare_build()
        self.mock('uname', 'echo Darwin\n')
        self.run_release('build', 'macos', '1.2.3')
        rust = self.root / 'matis-rust'
        import zipfile
        with zipfile.ZipFile(self.root / 'artifacts/matis-1.2.3-macos.zip') as archive:
            names = archive.namelist()
            for name in ['MacOS/matis', 'MacOS/matis-migrate', 'MacOS/matis-ui',
                         'Resources/dashboard/server/index.mjs', 'Info.plist']:
                self.assertIn(f'Matis.app/Contents/{name}', names)
            self.assertIn(b'1.2.3', archive.read('Matis.app/Contents/Info.plist'))
        self.assertIn('0.1.0', (rust / 'Cargo.toml').read_text())

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

    @unittest.skipUnless(Path('/etc/os-release').exists(), 'Linux packaging only')
    def test_linux_package_contains_cli_desktop_and_dashboard(self):
        import shlex
        release = dict(line.split('=', 1) for line in Path('/etc/os-release').read_text().splitlines() if '=' in line)
        ids = shlex.split(release['ID'] + ' ' + release.get('ID_LIKE', ''))
        platform = next((value for value in ['ubuntu', 'arch'] if value in ids), None)
        if platform is None:
            self.skipTest('Ubuntu or Arch required')
        self.prepare_build()
        (self.root / 'dashboard/public').mkdir()
        (self.root / 'dashboard/public/logo.png').write_bytes(b'icon')
        subprocess.run(['git', 'add', '.'], cwd=self.root, check=True)
        self.mock('makepkg', r'''
cp -a root "$FIXTURE/payload"
echo package > "matis-1.2.3-1-$(uname -m).pkg.tar.zst"
''')
        self.mock('dpkg', 'echo amd64\n')
        self.mock('dpkg-deb', r'''
cp -a "$3" "$FIXTURE/payload"
echo package > "$4"
''')
        self.run_release('build', platform, '1.2.3')
        payload = self.root / 'payload/usr'
        for name in ['bin/matis', 'bin/matis-migrate', 'bin/matis-ui',
                     'lib/matis/dashboard/server/index.mjs', 'share/applications/matis.desktop']:
            self.assertTrue((payload / name).is_file(), name)
        self.assertIn('Exec=matis-ui', (payload / 'share/applications/matis.desktop').read_text())
        self.assertEqual(len(list((self.root / 'artifacts').glob('matis-*'))), 1)


if __name__ == '__main__':
    unittest.main()
