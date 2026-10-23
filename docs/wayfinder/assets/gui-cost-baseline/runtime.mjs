// Reproducible, bounded normal-runtime probes on a runner-owned private Xvfb.
import fs from 'node:fs';
import path from 'node:path';
import assert from 'node:assert/strict';
import { spawn, execFileSync } from 'node:child_process';
import { fileURLToPath } from 'node:url';
const here=path.dirname(fileURLToPath(import.meta.url));
const quantile=(xs,q)=>[...xs].sort((a,b)=>a-b)[Math.max(0,Math.ceil(xs.length*q)-1)];
const stats=xs=>({n:xs.length,min:Math.min(...xs),median:quantile(xs,.5),p95:quantile(xs,.95),max:Math.max(...xs)});
assert.deepEqual(stats([4,1,3,2]),{n:4,min:1,median:2,p95:4,max:4});
if(process.argv[2]==='--self-test'){console.log('PASS: nearest-rank statistics self-check');process.exit(0);}
const root=path.resolve(process.argv[2]||'');
assert.ok(path.basename(root).startsWith('gui-cost-baseline.'));
assert.ok(process.env.DISPLAY && process.env.DISPLAY===process.env.GUI_BENCH_DISPLAY);
const bin=path.join(root,'stage/usr/local/bin/lite-xl');
assert.ok(fs.statSync(bin).isFile());
const tick=+execFileSync('getconf',['CLK_TCK'],{encoding:'utf8'}).trim();
const sleep=ms=>new Promise(r=>setTimeout(r,ms));
const now=()=>Number(process.hrtime.bigint())/1e6;
function proc(pid){
 try{
  const raw=fs.readFileSync(`/proc/${pid}/stat`,'utf8');
  const f=raw.slice(raw.lastIndexOf(')')+2).split(/\s+/);
  const status=fs.readFileSync(`/proc/${pid}/status`,'utf8');
  const get=name=>+(status.match(new RegExp(`^${name}:\\s+(\\d+)`,'m'))||[])[1];
  return {ticks:+f[11]+ +f[12],rss_kib:get('VmRSS'),vcs:get('voluntary_ctxt_switches'),ivcs:get('nonvoluntary_ctxt_switches')};
 }catch{return null;}
}
const idleOnly=process.argv.includes('--idle-only'), editOnly=process.argv.includes('--edit-only');
const resultsFile=path.join(root,idleOnly?'runtime-idle-results.json':editOnly?'runtime-edit-results.json':'runtime-results.json');
const out={started:new Date().toISOString(),clock_ticks:tick,display:process.env.DISPLAY,trials:[]};
fs.mkdirSync(path.join(root,'runtime'),{recursive:true});
async function trial(mode,variant,i){
 const id=`${mode}-${variant}-${i}`, dir=path.join(root,'runtime',id), user=path.join(dir,'user'), project=path.join(dir,'project');
 fs.mkdirSync(user,{recursive:true});fs.mkdirSync(project,{recursive:true});fs.mkdirSync(path.join(dir,'home'),{recursive:true});
 fs.copyFileSync(path.join(here,'probe.lua'),path.join(user,'init.lua'));
 fs.writeFileSync(path.join(user,'session.lua'),'return {recents={},window_mode="normal",window={1024,640,0,0}}\n');
 const line='local function ordinary(value) return value + 42 end -- representative Lua syntax\n';
 fs.writeFileSync(path.join(project,'fixture.lua'),line.repeat(variant==='large'?20000:2000));
 const env={...process.env,HOME:path.join(dir,'home'),XDG_CONFIG_HOME:path.join(dir,'home/config'),LITE_USERDIR:user,LITE_PREFIX:path.join(root,'stage/usr/local'),LITE_SCALE:'1',GUI_BENCH_ROOT:root,GUI_BENCH_MODE:mode,GUI_BENCH_VARIANT:variant,GUI_BENCH_FIXTURE:path.join(project,'fixture.lua')};
 for(const k of ['LITE_XL_RUNTIME','GDK_SCALE','QT_SCALE_FACTOR'])delete env[k];
 const start=now(), child=spawn(bin,[path.join(project,'fixture.lua')],{cwd:project,env,stdio:['ignore','pipe','pipe']});
 const data={id,mode,variant,frames:{},cycles:[],pid:child.pid,fixture_bytes:fs.statSync(path.join(project,'fixture.lua')).size};
 let pending='',stdout='',stderr='',timer,injectTimer;
 let readyResolve,readyReject;
 const ready=new Promise((r,j)=>{readyResolve=r;readyReject=j;});
 let injectionStart;
 const driver=action=>execFileSync(path.join(root,'focus'),[action],{env,timeout:2000});
 child.stdout.on('data',chunk=>{
  stdout+=chunk;pending+=chunk;
  let end;
  while((end=pending.indexOf('\n'))>=0){
   const line=pending.slice(0,end);pending=pending.slice(end+1);
   if(!line.startsWith('GUIBENCH\t'))continue;
   const [,kind,...values]=line.split('\t');
   if(kind==='timer_loop_ms')data.timer_loop_ms=+values[0];
   if(kind==='first_frame'){
    data.first_frame_observed_ms=now()-start;data.window=values.slice(1,3).map(Number);data.plugin_entries=+values[3];
    try{driver('focus');injectTimer=setTimeout(()=>{try{injectionStart=now();driver('tap');}catch(e){readyReject(e);child.kill('SIGKILL');}},30);}catch(e){readyReject(e);child.kill('SIGKILL');}
   }
   if(kind==='input_ready'){
    data.input_observed_ms=now()-start;data.xtest_to_lua_observed_ms=now()-injectionStart;data.focused=+values[1];readyResolve();
   }
   if(kind==='focus_lost'){data.focus_lost=+values[0];}
   if(kind==='edit_start')data.edit_started_observed_ms=now()-start;
   if(kind==='restarted_frame')data.edit_to_restarted_frame_ms=now()-start-data.edit_started_observed_ms;
   if(kind==='frame'){
    const [phase,...numbers]=values;(data.frames[phase]??=[]).push(numbers.map(Number));
   }
   if(kind==='cycle'){const p=proc(child.pid);if(p)data.cycles.push({cycle:+values[0],...p});}
  }
 });
 child.stderr.on('data',c=>stderr+=c);
 const exited=new Promise((resolve,reject)=>{
  timer=setTimeout(()=>{child.kill('SIGKILL');reject(new Error(`${id} exceeded 60 seconds`));},60000);
  child.on('error',reject);
  child.on('exit',(code,signal)=>{clearTimeout(timer);clearTimeout(injectTimer);resolve({code,signal});});
 });
 try{
  // Race readiness against early exit; no infinite wait after bootstrap failure.
  await Promise.race([ready,exited.then(e=>{if(!data.input_observed_ms)throw new Error(`${id} exited before input marker: ${JSON.stringify(e)}\n${stdout}\n${stderr}`);})]);
  assert.equal(data.focused,1,`${id}: focus handshake`);
  if(mode==='idle'){
   await sleep(250);
   data.idle=[];
   for(const focus of ['focused','unfocused']){
    if(focus==='unfocused'){driver('blur');await sleep(250);assert.equal(data.focus_lost,0,`${id}: native focus loss not observed`);}
    const a=proc(child.pid),t=now();assert.ok(a);
    await sleep(5000);
    const b=proc(child.pid),seconds=(now()-t)/1000;assert.ok(b);
    data.idle.push({focus,seconds,cpu_one_core_percent:(b.ticks-a.ticks)/tick/seconds*100,rss_before_kib:a.rss_kib,rss_after_kib:b.rss_kib,voluntary_switches_per_s:(b.vcs-a.vcs)/seconds,involuntary_switches_per_s:(b.ivcs-a.ivcs)/seconds});
   }
   child.kill('SIGTERM');
  }
  const exit=await exited;
  assert.equal(exit.code,0,`${id}: ${JSON.stringify(exit)}\n${stdout}\n${stderr}`);
  assert.ok(!fs.existsSync(path.join(user,'error.txt')),`${id}: error.txt`);
  if(mode==='frames')for(const phase of ['type','scroll','layout'])assert.equal(data.frames[phase]?.length,90,`${id}/${phase}`);
  if(mode==='retention')assert.ok(data.cycles.length>=50,`${id}: missing RSS samples`);
  if(mode==='edit')assert.ok(data.edit_to_restarted_frame_ms>0,`${id}: user-init save did not restart`);
  data.elapsed_ms=now()-start;
  const labels=['action_ms','update_ms','draw_commands_ms','cache_raster_present_ms','step_ms','action_to_step_end_ms'];
  data.frame_stats={};
  for(const [phase,rows] of Object.entries(data.frames))data.frame_stats[phase]=Object.fromEntries(labels.map((name,k)=>[name,stats(rows.map(r=>r[k]))]));
  fs.writeFileSync(path.join(dir,'stdout.txt'),stdout);fs.writeFileSync(path.join(dir,'stderr.txt'),stderr);
  fs.writeFileSync(path.join(dir,'raw.json'),JSON.stringify(data,null,2)+'\n');
  delete data.frames; // Per-frame samples retained separately; summary stays reviewable.
  out.trials.push(data);
  fs.writeFileSync(resultsFile,JSON.stringify(out,null,2)+'\n');
  console.log(id,'first_frame_ms=',data.first_frame_observed_ms.toFixed(2));
 }catch(e){
  child.kill('SIGKILL');clearTimeout(timer);clearTimeout(injectTimer);
  fs.writeFileSync(path.join(dir,'stdout.txt'),stdout);fs.writeFileSync(path.join(dir,'stderr.txt'),stderr);
  throw e;
 }
}
const smoke=process.argv.includes('--smoke');
if(idleOnly || editOnly){
 for(const variant of ['bundled','custom'])for(let i=1;i<=(idleOnly?2:3);i++)await trial(idleOnly?'idle':'edit',variant,i);
}else for(const variant of (smoke?['bundled']:['bundled','custom','large'])){
 for(let i=1;i<=(smoke?1:10);i++)await trial('startup',variant,i);
 for(let i=1;i<=(smoke?1:3);i++)await trial('frames',variant,i);
 if(!smoke && variant!=='large'){
  for(let i=1;i<=2;i++)await trial('idle',variant,i);
  for(let i=1;i<=3;i++)await trial('retention',variant,i);
 }
}
out.finished=new Date().toISOString();
fs.writeFileSync(resultsFile,JSON.stringify(out,null,2)+'\n');
console.log('PASS:',out.trials.length,'bounded runtime probes');
