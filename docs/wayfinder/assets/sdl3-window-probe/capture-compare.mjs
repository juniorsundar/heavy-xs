#!/usr/bin/env node
// THROWAWAY KDE/Wayland comparison. No clipboard, window movement or display changes.
import {spawn, spawnSync} from 'node:child_process';
import {readFileSync, writeFileSync, mkdirSync, openSync, closeSync, existsSync, unlinkSync} from 'node:fs';
import {dirname, resolve, join} from 'node:path';
import {fileURLToPath} from 'node:url';
import {createHash, randomBytes} from 'node:crypto';
import assert from 'node:assert/strict';
const here = dirname(fileURLToPath(import.meta.url)), root = resolve(here, '../../../..');
const usage = 'Usage: node capture-compare.mjs --run OUTPUT NEW_OUTDIR [--compare-opaque-hint]\n       node capture-compare.mjs --self-check\nNo args/--help: no actions. Live: two eight-second borderless cases with private\nprotocol logs and guarded probe-only captures. Default cases zero/ff; the final\n--compare-opaque-hint flag selects zero without/with the window transparent hint.\nRequires KWin, journalctl, qdbus, Spectacle, ImageMagick, Cargo; PROBE_NIX=1 opts\ninto Nix. No SSH. See PRESENTATION.md.';
const args = process.argv.slice(2);
const sleep = ms => new Promise(r => setTimeout(r, ms));
const source = readFileSync(join(here, 'capture-guard.js'), 'utf8');
function titleFor(padding, transparent) {
    return 'sdl3 window probe (throwaway) wayland-padding=' + padding + ' transparent=' + transparent;
}
function casesFor(compare) {
    const rows = compare
        ? [{name:'zero-default',padding:'zero',transparent:false},{name:'zero-transparent',padding:'zero',transparent:true}]
        : [{name:'zero',padding:'zero',transparent:false},{name:'ff',padding:'ff',transparent:false}];
    return rows.map(c => ({...c, title: titleFor(c.padding, c.transparent)}));
}
function parseCli(argv) {
    const compare = argv[3] === '--compare-opaque-hint';
    if (argv[0] !== '--run' || !argv[1] || !argv[2] || argv.length !== (compare ? 4 : 3)) return null;
    return {output: argv[1], dir: argv[2], compare};
}
function decode(line, marker) {
    const at = line.indexOf(marker + ' ');
    if (at < 0) return null;
    try { return JSON.parse(line.slice(at + marker.length + 1)); } catch { return null; }
}
function same(a, b) { return JSON.stringify(a) === JSON.stringify(b); }
function matches(s, output) {
    return s?.match === true && s.active === true && s.output === output && s.scale > 1 &&
        s.id && Number.isInteger(s.pid) && s.geometry?.length === 4 && s.geometry.every(Number.isFinite);
}
if (!args.length || args[0] === '--help') { console.log(usage); process.exit(0); }
if (args.length === 1 && args[0] === '--self-check') {
    assert.equal(decode('unrelated', 'MARK'), null);
    assert.deepEqual(decode('js: MARK {"kind":"pre","state":{"match":false}}', 'MARK'), {kind:'pre',state:{match:false}});
    const s = {match:true,active:true,output:'DP-8',scale:1.2,id:'uuid',pid:123,geometry:[0,0,800,520]};
    assert.ok(matches(s,'DP-8')); assert.ok(!matches({...s,active:false},'DP-8'));
    assert.ok(!matches(s,'eDP-1')); assert.ok(!matches({...s,scale:1},'DP-8'));
    assert.ok(same(s,{...s})); assert.ok(!same(s,{...s,id:'different'}));
    assert.ok(same({...s,armed:undefined},s));
    assert.ok(!same(s,{...s,geometry:[0,0,2543,2076]}));
    // Exercise the real observer against mocks: missing signals must fail closed.
    const execute = new Function('workspace','print',source.replace('__CAPTURE_CONFIG__',JSON.stringify({phase:'watch',marker:'MARK',pid:123,title:'probe'})));
    const messages=[], signal={connect(){}};
    const w={pid:123,caption:'probe',active:true,internalId:'uuid',output:{name:'DP-8',devicePixelRatio:1.2},frameGeometry:{x:0,y:0,width:800,height:520},frameGeometryChanged:signal,captionChanged:signal,outputChanged:signal};
    const workspace={activeWindow:w,windowActivated:signal};
    execute(workspace,line=>messages.push(decode(line,'MARK')));
    assert.equal(messages.at(-1).state.armed,true);
    delete w.outputChanged;
    execute(workspace,line=>messages.push(decode(line,'MARK')));
    assert.equal(messages.at(-1).kind,'error');
    assert.deepEqual(parseCli(['--run','DP-8','/tmp/new']), {output:'DP-8',dir:'/tmp/new',compare:false});
    assert.deepEqual(parseCli(['--run','DP-8','/tmp/new','--compare-opaque-hint']),
        {output:'DP-8',dir:'/tmp/new',compare:true});
    assert.equal(parseCli(['--compare-opaque-hint']), null);
    assert.equal(parseCli(['--run','DP-8']), null);
    assert.equal(parseCli(['--run','DP-8','/tmp/new','--compare-opaque-hint','extra']), null);
    assert.equal(parseCli(['--run','DP-8','/tmp/new','--other']), null);
    const defaults=casesFor(false), hint=casesFor(true);
    assert.deepEqual(defaults.map(c=>[c.name,c.padding,c.transparent]),
        [['zero','zero',false],['ff','ff',false]]);
    assert.deepEqual(hint.map(c=>[c.name,c.padding,c.transparent]),
        [['zero-default','zero',false],['zero-transparent','zero',true]]);
    assert.ok(hint.every(c=>c.padding==='zero'));
    assert.equal(defaults[0].title, titleFor('zero',false));
    assert.equal(hint[0].title, 'sdl3 window probe (throwaway) wayland-padding=zero transparent=false');
    assert.equal(hint[1].title, 'sdl3 window probe (throwaway) wayland-padding=zero transparent=true');
    assert.equal(hint[0].title, defaults[0].title);
    assert.notEqual(hint[0].title, hint[1].title);
    console.log('PASS pure capture guard/parser self-check; no processes started'); process.exit(0);
}
const cli = parseCli(args);
if (!cli) { console.error(usage); process.exit(2); }
const output = cli.output, dir = resolve(cli.dir), cases = casesFor(cli.compare);
let journal, probe, captureChild, currentPng, currentValid = false, createdDir = false;
const scripts = new Map(), events = [], marker = 'LXL_CAPTURE_' + randomBytes(8).toString('hex');
function cmd(exe, argv, timeout=10000) {
    const r = spawnSync(exe, argv, {encoding:'utf8',timeout});
    if (r.error || r.status !== 0) throw new Error(`${exe}: ${r.error?.message || r.stderr || r.status}`);
    return r.stdout.trim();
}
function dbus(path, method, ...argv) { return cmd('qdbus', ['org.kde.KWin',path,method,...argv]); }
function unload(name) {
    if (scripts.has(name)) { dbus('/Scripting','org.kde.kwin.Scripting.unloadScript',name); scripts.delete(name); }
}
function cleanup() {
    try { if (createdDir) writeFileSync(join(dir,'guard-events.json'),JSON.stringify(events,null,2)+'\n',{mode:0o600}); }
    catch { console.error('Could not preserve guard events'); process.exitCode=2; }
    if (captureChild) { try { process.kill(-captureChild.pid, 'SIGKILL'); } catch {} }
    if (currentPng && !currentValid && existsSync(currentPng)) unlinkSync(currentPng);
    for (const name of scripts.keys()) { try { unload(name); } catch { console.error('ERROR: could not unload observer '+name); process.exitCode=2; } }
    journal?.kill();
    if (probe) { try { process.kill(-probe.pid,'SIGKILL'); } catch {} }
}
for (const sig of ['SIGINT','SIGTERM']) process.on(sig,()=>{cleanup();process.exit(sig==='SIGINT'?130:143);});
async function waitFor(fn, ms, label) {
    const end = Date.now()+ms;
    while (Date.now()<end) { const v=fn(); if (v) return v; await sleep(30); }
    throw new Error('Timeout: '+label);
}
async function snapshot(phase, cfg={}) {
    const name = marker+'_'+phase, path=join(dir,name+'.js'), start=events.length;
    writeFileSync(path,source.replace('__CAPTURE_CONFIG__',JSON.stringify({marker,phase,...cfg})),{mode:0o600});
    const id=dbus('/Scripting','org.kde.kwin.Scripting.loadScript',path,name);
    if (!/^\d+$/.test(id)) throw new Error('Invalid KWin script id');
    scripts.set(name,id);
    dbus('/Scripting/Script'+id,'org.kde.kwin.Script.run');
    const e=await waitFor(()=>events.slice(start).find(e=>e.kind===phase),2000,phase+' observer');
    if (phase!=='watch') unload(name);
    return e.state;
}
async function capture(png) {
    const fd=openSync(join(dirname(png),'capture.log'),'w',0o600);
    const p=spawn('timeout',['5','spectacle','--activewindow','--background','--nonotify','--no-decoration','--no-shadow','--output',png],{detached:true,stdio:['ignore',fd,fd]});
    captureChild=p;
    closeSync(fd);
    const status=await new Promise((done,fail)=>{p.once('error',fail);p.once('exit',done);});
    captureChild=null;
    if (status!==0) throw new Error('Capture failed; inspect capture.log');
}
async function run() {
    if (process.env.SSH_CONNECTION || process.env.SSH_CLIENT || process.env.SSH_TTY) throw new Error('SSH session refused');
    if (process.env.XDG_SESSION_TYPE!=='wayland' || !process.env.WAYLAND_DISPLAY) throw new Error('Native Wayland session required');
    if (!output || existsSync(dir)) throw new Error('Need output name and NEW output directory');
    mkdirSync(dir,{mode:0o700}); createdDir=true;
    writeFileSync(join(dir,'metadata.txt'),[
        'commit='+cmd('git',['-C',root,'rev-parse','HEAD']),
        'lock_sha256='+createHash('sha256').update(readFileSync(join(here,'Cargo.lock'))).digest('hex'),
        'kwin='+cmd('kwin_wayland',['--version']), 'output='+output, 'date='+new Date().toISOString(),
        'cases='+cases.map(c=>`${c.name} padding=${c.padding} transparent=${c.transparent}`).join('; '),
        'guard=snapshots plus change notifications; not an atomic pinned-window capture'
    ].join('\n')+'\n',{mode:0o600});
    const build=['cargo','build','--locked','--offline','--manifest-path',join(here,'Cargo.toml')];
    const env={...process.env,CARGO_TARGET_DIR:join(here,'target')};
    const b=process.env.PROBE_NIX==='1' ? spawnSync('nix',['develop',root,'--command',...build],{env,stdio:'inherit',timeout:300000}) : spawnSync(build[0],build.slice(1),{env,stdio:'inherit',timeout:300000});
    if(b.error || b.status!==0) throw new Error('Offline prebuild failed');
    journal=spawn('journalctl',['--user','--follow','--lines=0','--output=cat','--quiet'],{stdio:['ignore','pipe','ignore']});
    let tail=''; journal.stdout.on('data',chunk=>{
        tail+=chunk; const lines=tail.split('\n'); tail=lines.pop();
        for(const line of lines) { const e=decode(line,marker); if(e) events.push(e); }
    });
    journal.on('error',error=>{console.error(error.message);});
    await sleep(300);
    const initial=await snapshot('output');
    if(initial.output!==output || !(initial.scale>1)) throw new Error(`Active output is ${initial.output} at ${initial.scale}; expected fractional ${output}. No probe launched.`);
    writeFileSync(join(dir,'output.json'),JSON.stringify(initial)+'\n',{mode:0o600});
    for (const item of cases) {
        const c=join(dir,item.name); mkdirSync(c,{mode:0o700}); currentPng=join(c,'window.png');currentValid=false;
        const log=join(c,'live.log'), a=openSync(log,'w',0o600), z=openSync(join(c,'wayland-debug.log'),'w',0o600);
        probe=spawn('bash',[join(here,'run.sh'),'--backend','wayland','--borderless',
            '--wayland-padding',item.padding,...(item.transparent?['--transparent-window']:[]),'--smoke-seconds','8'],
            {env:{...process.env,WAYLAND_DEBUG:'client'},detached:true,stdio:['ignore',a,z]});
        closeSync(a);closeSync(z);
        let status; probe.on('exit',code=>{status=code;probe=null;}); probe.on('error',error=>{status=-1;console.error(error.message);});
        const getLog=()=>readFileSync(log,'utf8');
        const pid=Number((await waitFor(()=>getLog().match(/probe: pid=(\d+)/),5000,'probe PID'))[1]);
        await waitFor(()=>getLog().includes('probe: presented frame'),3000,'first presented frame');
        const cfg={pid,title:item.title};
        const watched=await snapshot('watch',cfg);
        if(watched.armed!==true || !matches(watched,output) || watched.scale!==initial.scale) throw new Error('Probe not active on the agreed fractional output; no capture');
        // The tiling manager may resize after the first present: settle before arming capture.
        let stable=watched, settledGeometry=false;
        const end=Date.now()+2500;
        while(Date.now()<end) {
            await sleep(250);
            const next=await snapshot('stable',cfg);
            if(!matches(next,output) || next.id!==watched.id || next.scale!==initial.scale) throw new Error('Probe identity/output changed while settling');
            if(same({...stable,armed:undefined},next)) { stable=next; settledGeometry=true; break; }
            stable=next;
        }
        if(!settledGeometry) throw new Error('Probe geometry did not settle');
        const start=events.length;
        const pre=await snapshot('pre',cfg);
        if(!same(stable,pre)) throw new Error('Probe changed before capture');
        await capture(currentPng);
        const post=await snapshot('post',cfg);
        await sleep(200);
        // A final KWin journal message is an ordering barrier, not an atomic capture.
        const settled=await snapshot('settled',cfg);
        if(!same(pre,post) || !same(pre,settled) || events.slice(start).some(e=>e.kind==='changed' || e.kind==='error')) throw new Error('Capture identity/focus/geometry changed; capture rejected');
        const frames=[...getLog().matchAll(/probe: presented frame (\d+)x(\d+) at scale ([\d.]+)/g)];
        const last=frames.at(-1), expected=last?.[1]+'x'+last?.[2];
        const dims=cmd('magick',['identify','-format','%wx%h',currentPng]);
        if(dims!==expected || Math.abs(Number(last?.[3])-pre.scale)>0.001) throw new Error('Capture does not match presented dimensions/scale');
        // KWin frame geometry is logical; captured pixels are physical.
        if(Math.abs(pre.geometry[2]*pre.scale-Number(last[1]))>2 || Math.abs(pre.geometry[3]*pre.scale-Number(last[2]))>2) throw new Error('Capture does not match guarded window extent');
        currentValid=true;
        const colors=Number(cmd('magick',['identify','-format','%k',currentPng]));
        const strip=cmd('magick',[currentPng,'-crop',`1x1+${Math.floor(Number(last[1])/2)}+${Math.floor(20*pre.scale)}`,'-alpha','off','-depth','8','txt:-']);
        const visible=colors>=10 && /#244C3A/i.test(strip);
        writeFileSync(join(c,'result.json'),JSON.stringify({name:item.name,padding:item.padding,transparent:item.transparent,
            ...pre,presented:expected,capture:dims,colors,visible},null,2)+'\n',{mode:0o600});
        unload(marker+'_watch');
        await waitFor(()=>status!==undefined,10000,'probe exit');
        if(status!==0) throw new Error('Probe exited '+status);
        probe=null;console.log(item.name+': '+(visible?'VISIBLE':'BLANK/UNEXPECTED PIXELS')+' '+dims+' scale='+pre.scale);
        await sleep(400);
    }
    const [first, second] = cases;
    const ae=spawnSync('magick',['compare','-metric','AE',join(dir,first.name,'window.png'),join(dir,second.name,'window.png'),'null:'],{encoding:'utf8',timeout:10000});
    if(ae.error || ![0,1].includes(ae.status)) throw new Error('Image comparison failed');
    const intent = cli.compare
        ? 'both zero padding; winit with_transparent(false) vs with_transparent(true) opaque-region hint change; not a supported remedy'
        : 'same window without the transparent hint; XRGB8888 high byte zero vs ff';
    writeFileSync(join(dir,'comparison.txt'),[
        'first='+first.name+' padding='+first.padding+' transparent='+first.transparent,
        'second='+second.name+' padding='+second.padding+' transparent='+second.transparent,
        'pixel_AE='+ae.stderr.trim(), 'intent='+intent].join('\n')+'\n',{mode:0o600});
    console.log('Pixel comparison AE='+ae.stderr.trim()+'; artifacts: '+dir+' (private). '+intent+'.');
}
try {await run();} catch(e) {
    if(createdDir) writeFileSync(join(dir,'failure.txt'),e.message+'\n',{mode:0o600});
    console.error('capture-compare: '+e.message);process.exitCode=2;
} finally {cleanup();}
