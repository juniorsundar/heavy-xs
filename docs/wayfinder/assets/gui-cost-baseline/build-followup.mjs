// Evidence-only: run against the owned experiment directory, never the main checkout.
import fs from 'node:fs';
import path from 'node:path';
import assert from 'node:assert/strict';
import { spawnSync, execFileSync } from 'node:child_process';
const root = path.resolve(process.argv[2] || '');
assert.ok(path.basename(root).startsWith('gui-cost-baseline.'));
const source = path.join(root, 'normal-source');
assert.ok(fs.statSync(path.join(source, '.git')).isDirectory());
execFileSync('git', ['diff', '--exit-code'], { cwd: source });
const input = path.join(source, 'src/system.rs');
const original = fs.readFileSync(input, 'utf8');
const needle = 'Ok(ticks as f64 / 1_000_000_000.0)';
assert.equal(original.split(needle).length, 2);
const results = { started: new Date().toISOString(), source: execFileSync('git', ['rev-parse', 'HEAD'], {cwd: source, encoding:'utf8'}).trim(), trials: [] };
const logs = path.join(root, 'logs/normal-checkout');
fs.mkdirSync(logs, {recursive:true});
function build(id, profile) {
  const target = path.join(root, 'build', profile === 'debug' ? 'target-debug' : 'target-release');
  const args = ['build','--offline','--locked','-j','4', ...(profile === 'release' ? ['--release'] : [])];
  const log = fs.openSync(path.join(logs, `${id}.log`), 'w');
  const timefile = path.join(logs, `${id}.time.json`);
  const result = spawnSync('/run/current-system/sw/bin/time', ['-f','{"wall_s":%e,"user_s":%U,"sys_s":%S,"max_rss_kib":%M}', '-o', timefile, 'timeout','300','cargo', ...args], {cwd:source, env:{...process.env,CARGO_TARGET_DIR:target}, stdio:['ignore',log,log], timeout:310000});
  fs.closeSync(log);
  assert.equal(result.status, 0, `${id}: ${result.error || result.signal || result.status}`);
  const metrics = JSON.parse(fs.readFileSync(timefile,'utf8'));
  assert.ok(metrics.wall_s >= 0 && metrics.max_rss_kib > 0);
  results.trials.push({id,profile,args,...metrics});
  fs.writeFileSync(path.join(root,'normal-build-results.json'),JSON.stringify(results,null,2)+'\n');
  const bytes = +execFileSync('du',['-sb',path.join(root,'build')],{encoding:'utf8'}).split(/\s/)[0];
  assert.ok(bytes < 15*1024**3, 'owned artifacts exceeded 15 GiB');
}
try {
  for (const profile of ['debug','release']) build(`relocate-${profile}`,profile);
  for (const profile of ['debug','release']) for (let i=1;i<=3;i++) build(`noop-${profile}-${i}`,profile);
  for (let trial=1;trial<=2;trial++) {
    const expression = trial === 1 ? 'Ok((ticks as f64 / 1_000_000_000.0) + 0.0)' : 'Ok((ticks as f64 / 1_000_000_000.0) * 1.0)';
    fs.writeFileSync(input, original.replace(needle, expression));
    for (const profile of ['debug','release']) build(`edit-${profile}-${trial}`,profile);
    fs.writeFileSync(input,original);
    for (const profile of ['debug','release']) build(`restore-${profile}-${trial}`,profile);
  }
} finally {
  fs.writeFileSync(input, original);
  execFileSync('git',['diff','--exit-code'],{cwd:source});
  results.finished = new Date().toISOString();
  fs.writeFileSync(path.join(root,'normal-build-results.json'),JSON.stringify(results,null,2)+'\n');
}
console.log(JSON.stringify(results));
