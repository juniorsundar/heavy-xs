// Evidence-only runner. Nothing third-party executes until all isolation gates pass.
// shortcut: this machine's Nix paths and reference hashes; regenerate after a reviewed environment change.
import fs from 'node:fs';
import path from 'node:path';
import assert from 'node:assert/strict';
import crypto from 'node:crypto';
import {spawn, execFileSync} from 'node:child_process';
import {fileURLToPath} from 'node:url';
const here=path.dirname(fileURLToPath(import.meta.url));
const sha=p=>crypto.createHash('sha256').update(fs.readFileSync(p)).digest('hex');
const output=(cmd,args)=>execFileSync(cmd,args,{encoding:'utf8',timeout:15000}).trim();
const markers=s=>s.split('\n').filter(l=>l.startsWith('GUIREF\t')).map(l=>l.split('\t').slice(1));
assert.deepEqual(markers('noise\nGUIREF\tPASS\tx\ty\n'),[['PASS','x','y']]);
if(process.argv[2]==='--self-test'){console.log('PASS: marker parser self-check');process.exit(0);}
const root=path.resolve(process.argv[2]||'');
assert.ok(path.basename(root).startsWith('gui-compat-reference.'));
const prefix=path.resolve(process.argv[3]||'');
assert.equal(sha(path.join(prefix,'bin/lite-xl')),'bde5ae899aef0302c41201ad54569f045bdef0db6a46874bd0a6f835642cb186');
const source='e556d388892280d8343a82a625941137e3a6ac5f';
assert.equal(output('git',['rev-parse','HEAD']),source);
execFileSync('git',['diff','--quiet','--','src','data','docs/api','Cargo.toml','Cargo.lock']);
const tracked=new Set(output('git',['ls-tree','-r','--name-only',source,'--','data','docs/api']).split('\n'));
const runtimeData={};
function checkData(dir,relative=''){
 for(const name of fs.readdirSync(dir)){
  const file=path.join(dir,name),rel=path.join(relative,name);
  if(fs.statSync(file).isDirectory()){checkData(file,rel);continue;}
  const original=tracked.has('data/'+rel)?'data/'+rel:'docs/api/'+rel;
  assert.ok(tracked.has(original),'unexpected staged runtime file: '+rel);
  let bytes=fs.readFileSync(original);
  if(rel==='core/start.lua')bytes=Buffer.from(bytes.toString().replaceAll('@PROJECT_VERSION@','2.1.7'));
  assert.equal(sha(file),crypto.createHash('sha256').update(bytes).digest('hex'),rel);
  runtimeData[rel]=sha(file);
 }
}
checkData(path.join(prefix,'share/lite-xl'));
const display=process.env.DISPLAY, auth=process.env.XAUTHORITY;
assert.match(display||'',/^:\d+$/);
assert.equal(display,process.env.GUI_REFERENCE_DISPLAY);
assert.ok(auth && path.resolve(auth).startsWith(root+path.sep),'Xauthority must be runner-owned');
const socket='/tmp/.X11-unix/X'+display.slice(1);
assert.ok(fs.statSync(socket).isSocket());
const bwrap='/nix/store/zri6pfys37ih88s5yckr47khz5zw1rzk-bubblewrap-0.12.0/bin/bwrap';
const prlimit=output('which',['prlimit']);
const helper=path.join(root,'isolation');
assert.ok(fs.statSync(helper).isFile());
const manifest=JSON.parse(fs.readFileSync(path.join(root,'verified/manifest.json')));
const pluginDir=path.join(root,'verified/plugins');
const names=['autosaveonfocuslost','bigclock','bracketmatch','custom_caret','indentguide','scalestatus','smoothcaret'];
const expected={autosaveonfocuslost:'acb7bc697b223c9cad8dc75f387df915d743756fe5df6cd6c9aba310d402d364',bigclock:'7a992ac48994fbcca8b3044b9e20b87eb87caeb9e3d98930d9600b4d1d32a2ce',bracketmatch:'c85dbe1c60c77612dd5bac17cb0e619cfc3e1a798933a9f1ca2497ccd826c6b6',indentguide:'a5b9688234cda8b2e4ddb2d2b894834ca13d1bcf92c14d5ee793aec4aaba2519',scalestatus:'88d40fc37a849713ec0ecfdcac02bf3c4381e24d8e2135e86484b867374e1bda'};
expected.custom_caret='e76eda17f4b6734a366590e4c9780d3adda15b7890d9240fcfeff22333439b77';
expected.smoothcaret='c95c4bb0770a6e8a71bd946850d92e189e2ee3c5396b5072d375967c2d39c8bc';
const additional=JSON.parse(fs.readFileSync(path.join(root,'verified/followup-manifest.json')));
for(const id of names)assert.equal(sha(path.join(pluginDir,id+'.lua')),expected[id],id);
// Only runtime-library package closures, not the entire store or the developer's RPATH.
const roots=new Set();
for(const binary of [path.join(prefix,'bin/lite-xl'),helper,prlimit]){
 if(binary.startsWith('/nix/store/'))roots.add(binary.split('/').slice(0,4).join('/'));
 for(const line of output('ldd',[binary]).split('\n'))for(const m of line.matchAll(/\/nix\/store\/[a-z0-9]{32}-[^/\s]+/g))roots.add(m[0]);
}
const rpath=output('patchelf',['--print-rpath',path.join(prefix,'bin/lite-xl')]).split(':');
for(const dir of rpath)if(dir.startsWith('/nix/store/'))roots.add(dir.split('/').slice(0,4).join('/'));
const closure=[...new Set(output('nix-store',['-qR',...roots]).split('\n'))].sort();
assert.ok(closure.length<150);
for(const p of closure)assert.match(p,/^\/nix\/store\/[a-z0-9]{32}-[^/]+$/);
fs.writeFileSync(path.join(root,'runtime-closure.txt'),closure.join('\n')+'\n');
const canary=path.join(root,'host-only-canary');fs.writeFileSync(canary,'host-only\n');
const hostNet=fs.readlinkSync('/proc/self/ns/net');
const result={started:new Date().toISOString(),source,runtime_data_sha256:runtimeData,binary_sha256:sha(path.join(prefix,'bin/lite-xl')),plugin_manifest:manifest,additional_plugin_manifest:additional,display,screen:'1280x800x24 dpi96; window1024x640 scale1',closure,excluded_rpath:rpath.filter(p=>!p.startsWith('/nix/store/')),helpers:Object.fromEntries(['probe.lua','isolation.c','run.mjs'].map(p=>[p,sha(path.join(here,p))])),focus_helper_sha256:sha(path.join(root,'focus')),focus_source_sha256:sha(path.join(here,'../gui-cost-baseline/focus.c')),limits:{wall_s:45,kill_after_s:5,cpu_s:20,address_space_bytes:2147483648,nproc:128,open_files:256,file_bytes:16777216,work_tmpfs_bytes:134217728,tmp_tmpfs_bytes:16777216,output_bytes:16777216},gates:[],trials:[]};
fs.mkdirSync(path.join(root,'runs'),{recursive:true});
async function run(id,plugins,command=['/helpers/isolation','--launch'],displayValue=display){
 const dir=path.join(root,'runs',id);assert.ok(!fs.existsSync(dir),'use fresh IDs/root; do not reuse writable inputs');
 fs.mkdirSync(path.join(dir,'plugins'),{recursive:true});
 for(const p of plugins)fs.copyFileSync(path.join(pluginDir,p+'.lua'),path.join(dir,'plugins',p+'.lua'));
 fs.writeFileSync(path.join(dir,'session.lua'),'return {recents={},window_mode="normal",window={1024,640,0,0}}\n');
 fs.writeFileSync(path.join(dir,'fixture.lua'),'local function reference(value)\n  return (value + 1)\nend\n');
 const args=['--unshare-all','--unshare-user','--disable-userns','--new-session','--die-with-parent','--cap-drop','ALL','--clearenv'];
 for(const p of closure)args.push('--ro-bind',p,p);
 args.push('--proc','/proc','--dev','/dev','--size','16777216','--tmpfs','/tmp',
  '--dir','/tmp/.X11-unix','--ro-bind',socket,socket,
  '--size','134217728','--tmpfs','/work','--dir','/work/home','--dir','/work/run',
  '--dir','/work/user','--dir','/work/project','--dir','/helpers',
  '--ro-bind',prefix,'/opt','--ro-bind',helper,'/helpers/isolation',
  '--ro-bind',dir,'/input','--ro-bind',path.join(here,'probe.lua'),'/work/user/init.lua',
  '--ro-bind',path.join(dir,'plugins'),'/work/user/plugins','--ro-bind',auth,'/work/Xauthority',
  '--chdir','/work/project');
 const env={HOME:'/work/home',PATH:'/opt/bin',XDG_RUNTIME_DIR:'/work/run',XDG_CONFIG_HOME:'/work/home/config',
  XAUTHORITY:'/work/Xauthority',LITE_USERDIR:'/work/user',LITE_PREFIX:'/opt',LITE_SCALE:'1',
  DBUS_SESSION_BUS_ADDRESS:'unix:path=/work/no-bus',GUI_REFERENCE_ROOT:'/work',GUI_REFERENCE_PLUGINS:plugins.join(','),LANG:'C.UTF-8'};
 if(displayValue)env.DISPLAY=displayValue;
 for(const [k,v] of Object.entries(env))args.push('--setenv',k,v);
 args.push(prlimit,'--core=0','--as=2147483648','--nproc=128','--nofile=256','--fsize=16777216','--cpu=20','--',...command);
 fs.writeFileSync(path.join(dir,'sandbox-args.json'),JSON.stringify(args,null,2)+'\n');
 const execution=await new Promise(resolve=>{
  const child=spawn('timeout',['-k','5','45',bwrap,...args],{stdio:['ignore','pipe','pipe']});
  let stdout='',stderr='',error,focused=false;
  const timer=setTimeout(()=>{error='outer deadline';child.kill('SIGKILL');},55000);
  child.stdout.on('data',chunk=>{
   stdout+=chunk;
   if(Buffer.byteLength(stdout)>16777216){error='stdout limit';child.kill('SIGKILL');return;}
   if(!focused && stdout.includes('GUIREF\tREADY\tfocus\t')){
    focused=true;
    try{execFileSync(path.join(root,'focus'),['focus'],{timeout:2000,env:{...process.env,DISPLAY:display,XAUTHORITY:auth,GUI_BENCH_DISPLAY:display}});}
    catch(e){error='private focus helper: '+e.message;child.kill('SIGKILL');}
   }
  });
  child.stderr.on('data',chunk=>{stderr+=chunk;if(Buffer.byteLength(stderr)>16777216){error='stderr limit';child.kill('SIGKILL');}});
  child.on('error',e=>{error=e.message;});
  child.on('close',(status,signal)=>{clearTimeout(timer);resolve({stdout,stderr,error,status,signal});});
 });
 fs.writeFileSync(path.join(dir,'stdout.log'),execution.stdout);fs.writeFileSync(path.join(dir,'stderr.log'),execution.stderr);
 return {id,plugins,exit:execution.status,signal:execution.signal,error:execution.error,markers:markers(execution.stdout),stderr:execution.stderr,stdout:execution.stdout};
}
function save(){fs.writeFileSync(path.join(root,'results.json'),JSON.stringify(result,null,2)+'\n');}
try{
 const isolation=await run('isolation',[],['/helpers/isolation',canary,path.resolve('Cargo.toml'),hostNet]);
 result.gates.push(isolation);save();
 assert.equal(isolation.exit,0,isolation.stderr);assert.match(isolation.stdout,/ISOLATION filesystem=private network=private pid=private env=clean bounds=set/);
 const noDisplay=await run('no-display',[],undefined,'');result.gates.push(noDisplay);save();
 assert.equal(noDisplay.exit,1);assert.ok(!noDisplay.error,noDisplay.error);
 assert.match(noDisplay.stderr,/neither WAYLAND_DISPLAY nor WAYLAND_SOCKET nor DISPLAY is set/);
 assert.ok(!noDisplay.markers.some(m=>m[0]==='DONE'),'missing-display gate unexpectedly ran GUI');
 const baseline=await run('baseline',[]);result.gates.push(baseline);save();
 assert.equal(baseline.exit,0,baseline.stderr);assert.ok(!baseline.error,baseline.error);
 assert.ok(!baseline.markers.some(m=>['FAIL','ERROR'].includes(m[0])),baseline.stdout);
 assert.ok(baseline.markers.some(m=>m[0]==='DONE'),'trusted baseline did not finish');
 console.log('PASS: filesystem/network/PID/environment/resource gates; no-display negative; normal-core baseline');
 if(!process.argv.includes('--gates-only')){
  const cases=names.map(n=>[n,[n]]).concat([
   ['drawing-combination',['bracketmatch','indentguide']],
   ['caret-combination',['custom_caret','smoothcaret']],
   ['working-five',['autosaveonfocuslost','bigclock','custom_caret','scalestatus','smoothcaret']],
   ['initial-five',['autosaveonfocuslost','bigclock','bracketmatch','indentguide','scalestatus']]
  ]);
  for(const [id,plugins] of cases){
   const trial=await run(id,plugins);trial.passed=trial.exit===0&&!trial.error&&trial.markers.some(m=>m[0]==='DONE')&&!trial.markers.some(m=>['FAIL','ERROR'].includes(m[0]));
   result.trials.push(trial);save();console.log(`${trial.passed?'PASS':'REFERENCE FAILURE'}: ${id}`);
  }
 }
 result.finished=new Date().toISOString();save();
}catch(error){result.blocker=String(error);save();throw error;}
