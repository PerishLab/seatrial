import hashlib
import json
import pathlib
import re
import signal
import subprocess
import sys
import time

PLUMB = pathlib.Path('/opt/tools/plumb')
ROOT = pathlib.Path('/trial/source')
EVIDENCE = pathlib.Path('/trial/evidence')
STATE = pathlib.Path(sys.argv[2])
OBSERVED = []


def command(name, *arguments, budget=30):
    return [str(PLUMB), 'cache', 'run', '--json', '--cwd', str(ROOT),
            '--output', str(EVIDENCE / name), '--timeout', str(budget), '--', *arguments]


def observe(name, value, expected):
    assert value.returncode == expected, {'name': name, 'code': value.returncode, 'stdout': value.stdout, 'stderr': value.stderr}
    output = EVIDENCE / name
    report = json.loads((output / 'result.json').read_text())
    invocation = json.loads((output / 'invocation.json').read_text())
    assert report['schema'] == 'plumb.cache-run/v1', report
    assert invocation['schema'] == 'plumb.cache-run-invocation/v1', invocation
    assert report['exit'] == expected, report
    record = {'name': name, 'report': report, 'invocation': invocation}
    for stream in ['stdout', 'stderr']:
        data = (output / (stream + '.log')).read_text()
        assert len(data.encode()) < 32768, (name, stream)
        record[stream] = data
    OBSERVED.append(record)
    return record


def settled(record):
    assert record['report']['cleanup'], record
    for field in ['directory', 'registry']:
        assert not pathlib.Path(record['invocation'][field]).exists(), record


def run(name, *arguments, expected=0, budget=30):
    value = subprocess.run(command(name, *arguments, budget=budget), capture_output=True,
                           text=True, timeout=budget + 15)
    record = observe(name, value, expected)
    settled(record)
    return record


def gone(pid):
    path = pathlib.Path('/proc') / str(pid) / 'stat'
    try:
        fields = path.read_text().rsplit(') ', 1)[1].split()
    except FileNotFoundError:
        return
    assert fields[0] in ['Z', 'X'], (pid, fields)


def source():
    ROOT.mkdir()
    (ROOT / 'src').mkdir()
    EVIDENCE.mkdir()
    (ROOT / 'Cargo.toml').write_text('[package]\nname="trial"\nversion="0.0.0"\nedition="2024"\n')
    (ROOT / 'Cargo.lock').write_text('version=4\n[[package]]\nname="trial"\nversion="0.0.0"\n')
    (ROOT / 'src/lib.rs').write_text('''#[test]
fn proof() {
    let target = std::env::var_os("CARGO_TARGET_DIR").expect("owned target");
    let mut paths = vec![std::path::PathBuf::from(target)];
    let mut bytes = 0;
    while let Some(path) = paths.pop() {
        for item in std::fs::read_dir(path).unwrap() {
            let item = item.unwrap();
            let meta = item.metadata().unwrap();
            if meta.is_dir() { paths.push(item.path()); } else { bytes += meta.len(); }
        }
    }
    assert!(bytes > 0);
    println!("scope-bytes={bytes}");
}
''')
    (ROOT / 'target').mkdir()
    (ROOT / 'target/foreign').write_text('preserve')
    STATE.mkdir()
    (STATE / 'foreign').write_text('preserve')
    return {str(path.relative_to(ROOT)): hashlib.sha256(path.read_bytes()).hexdigest()
            for path in ROOT.rglob('*') if path.is_file()}


def lifecycle():
    cargo = run('cargo', 'cargo', 'test', '--locked', '--offline', '--', '--nocapture')
    assert '1 passed' in cargo['stdout'], cargo
    measured = re.search(r'scope-bytes=(\d+)', cargo['stdout'])
    assert measured is not None, cargo
    cargo['compiled_bytes'] = int(measured.group(1))
    assert cargo['compiled_bytes'] > 0, cargo
    run('failure', '/bin/sh', '-c', 'printf diagnostic >&2; exit 23', expected=23)
    run('signal', '/bin/sh', '-c', 'kill -KILL $$', expected=137)
    run('timeout', '/bin/sh', '-c', 'sleep 30', expected=124, budget=1)
    record = run('background', '/bin/sh', '-c', 'sleep 30 & echo $!; exit 0')
    gone(int(record['stdout'].strip()))
    values = ['$HOME', 'space value', '%n', '$(touch escaped)']
    record = run('literal', '/bin/sh', '-c', 'printf "%s\\n" "$@"', '--', *values)
    assert record['stdout'] == '\n'.join(values) + '\n', record
    assert not (ROOT / 'escaped').exists()


def cancellation():
    with subprocess.Popen(command('left', '/bin/sh', '-c', 'echo ready; sleep 30', budget=10),
                          stdout=subprocess.PIPE, stderr=subprocess.PIPE, text=True) as left:
        with subprocess.Popen(command('right', '/bin/sh', '-c', 'sleep 1; exit 0', budget=10),
                              stdout=subprocess.PIPE, stderr=subprocess.PIPE, text=True) as right:
            until = time.monotonic() + 5
            path = EVIDENCE / 'left/stdout.log'
            while not path.exists() or 'ready' not in path.read_text():
                assert time.monotonic() < until and left.poll() is None
                time.sleep(0.02)
            left.send_signal(signal.SIGTERM)
            out, err = left.communicate(timeout=15)
            settled(observe('left', subprocess.CompletedProcess(left.args, left.returncode, out, err), 143))
            out, err = right.communicate(timeout=15)
            settled(observe('right', subprocess.CompletedProcess(right.args, right.returncode, out, err), 0))


def refusal():
    output = EVIDENCE / 'foreign'
    output.mkdir()
    (output / 'marker').write_text('preserve')
    value = subprocess.run(command('foreign', '/bin/true'), capture_output=True, text=True, timeout=15)
    assert value.returncode == 125, value
    assert (output / 'marker').read_text() == 'preserve'
    environment = {'PATH': '/opt/tools:/usr/bin:/bin', 'HOME': sys.argv[3],
                   'PLUMB_HOME': str(STATE), 'CARGO_TARGET_DIR': str(ROOT / 'target')}
    value = subprocess.run(command('bound', '/bin/true'), env=environment,
                           capture_output=True, text=True, timeout=15)
    assert value.returncode == 125 and not (EVIDENCE / 'bound').exists(), value
    value = subprocess.run(command('changed', '/bin/sh', '-c',
                                 'printf foreign >"$PLUMB_HOME/../plumb.json"'),
                           capture_output=True, text=True, timeout=15)
    record = observe('changed', value, 125)
    assert record['report']['process'] and not record['report']['cleanup'], record
    for field in ['directory', 'registry']:
        assert pathlib.Path(record['invocation'][field]).exists(), record


def main():
    version = subprocess.check_output([PLUMB, '--version'], text=True).strip()
    assert version == 'plumb ' + sys.argv[1], version
    before = source()
    lifecycle()
    cancellation()
    refusal()
    after = {str(path.relative_to(ROOT)): hashlib.sha256(path.read_bytes()).hexdigest()
             for path in ROOT.rglob('*') if path.is_file()}
    assert before == after, (before, after)
    assert (STATE / 'foreign').read_text() == 'preserve'
    result = {'schema': 'seatrial.plumb-cache-run/v1', 'version': version,
              'binary': hashlib.sha256(PLUMB.read_bytes()).hexdigest(),
              'source': before, 'operations': OBSERVED}
    text = json.dumps(result, sort_keys=True)
    assert len(text.encode()) < 128 * 1024
    print(text)


main()
