#!/usr/bin/env python3
"""İzole Tauri/WebKit sürecinin PSS/RSS eğilimi; normal FocusFlow'a dokunmaz."""
import os,time,json,subprocess,tempfile,threading,math
from pathlib import Path
import gi
gi.require_version('Atspi','2.0')
from gi.repository import Atspi
root=Path(__file__).resolve().parent.parent
out=root/'audit/2026-10-22'
out.mkdir(parents=True,exist_ok=True)
temp=Path(tempfile.mkdtemp(prefix='focusflow-memory-'))
binary=Path(os.environ.get('FOCUSFLOW_AUDIT_BINARY',str(out/'focusflow-audit-fixed')))
seed=root/'src-tauri/target/debug/examples/audit_probe'
db=temp/'data/com.focusflow.audit20261022/focusflow.db'
subprocess.run([str(seed),'--seed',str(db),'1000'],check=True)
env={**os.environ,'XDG_DATA_HOME':str(temp/'data'),'XDG_CONFIG_HOME':str(temp/'config'),'NO_AT_BRIDGE':'0','GTK_MODULES':'gail:atk-bridge'}
log=(out/'native-memory-app.log').open('w')
p=subprocess.Popen([str(binary)],env=env,stdout=log,stderr=log)
stop=threading.Event();samples=[];phase='warmup';start=time.monotonic();socket_observations=set()
def descendants(pid):
    found={pid};todo=[pid]
    while todo:
        n=todo.pop()
        try:
            for v in Path(f'/proc/{n}/task/{n}/children').read_text().split():
                c=int(v)
                if c not in found:found.add(c);todo.append(c)
        except OSError:pass
    return found

def sampler():
    with (out/'native-memory-samples.jsonl').open('w') as f:
        last_heartbeat=-30
        while not stop.is_set():
            if p.poll() is not None:break
            totals={'pss_kb':0,'rss_kb':0,'private_dirty_kb':0,'swap_kb':0,'measured_processes':0,'swap_pss_kb':0,'anon_pss_kb':0};pids=descendants(p.pid);inodes=set()
            for pid in pids:
                try:
                    stats={l.split(':')[0]:int(l.split()[1]) for l in Path(f'/proc/{pid}/smaps_rollup').read_text().splitlines() if ':' in l and l.split()[1].isdigit()}
                    totals['measured_processes']+=1
                    for key,source in [('pss_kb','Pss'),('rss_kb','Rss'),('private_dirty_kb','Private_Dirty'),('swap_kb','Swap'),('swap_pss_kb','SwapPss'),('anon_pss_kb','Pss_Anon')]:totals[key]+=stats.get(source,0)
                    for fd in Path(f'/proc/{pid}/fd').iterdir():
                        try:
                            target=os.readlink(fd)
                            if target.startswith('socket:['):inodes.add(target[8:-1])
                        except OSError:pass
                except OSError:pass
            for kind in ('tcp','tcp6','udp','udp6'):
                try:
                    for line in Path(f'/proc/{p.pid}/net/{kind}').read_text().splitlines()[1:]:
                        cols=line.split()
                        if len(cols)>9 and cols[9] in inodes:socket_observations.add((kind,cols[1],cols[2],cols[3]))
                except OSError:pass
            totals['footprint_kb']=totals['pss_kb']+totals['swap_pss_kb'];totals['anonymous_footprint_kb']=totals['anon_pss_kb']+totals['swap_pss_kb']
            item={'seconds':round(time.monotonic()-start,2),'phase':phase,'processes':len(pids),**totals};samples.append(item);f.write(json.dumps(item)+'\n');f.flush()
            if item['seconds']-last_heartbeat>=20:
                print(f"Bellek ölçümü: {phase}, {item['seconds']:.0f} sn, PSS {item['pss_kb']/1024:.1f} MiB",flush=True);last_heartbeat=item['seconds']
            stop.wait(2)

def wait(fn,timeout=30):
    until=time.monotonic()+timeout
    while time.monotonic()<until:
        if p.poll() is not None:raise RuntimeError('Test uygulaması kapandı: '+str(p.returncode))
        result=fn()
        if result:return result
        time.sleep(.1)
    raise TimeoutError('Yerel test denetimi bulunamadı')

def app_for_pid():
    d=Atspi.get_desktop(0)
    for i in range(d.get_child_count()):
        a=d.get_child_at_index(i)
        try:
            if a.get_process_id()==p.pid:return a
        except Exception:pass

def walk(obj,depth=0):
    if depth>22:return
    yield obj
    try:
        for i in range(min(obj.get_child_count(),200)):
            c=obj.get_child_at_index(i)
            if c:yield from walk(c,depth+1)
    except Exception:pass

def button(name):
    for o in walk(app):
        try:
            if o.get_name()==name and o.get_action_iface() and o.get_action_iface().get_n_actions()>0:return o
        except Exception:pass

def click(name):
    b=wait(lambda:button(name));assert b.get_action_iface().do_action(0)

def tail_stats(name):
    values=[s for s in samples if s['phase']==name and s['measured_processes']>=3]
    if not values:return {}
    values=values[-30:];n=len(values);xs=[s['seconds'] for s in values];ys=[s['pss_kb']/1024 for s in values];mx=sum(xs)/n;my=sum(ys)/n;den=sum((x-mx)**2 for x in xs)
    return {'samples':n,'pss_mean_mb':round(my,2),'pss_min_mb':round(min(ys),2),'pss_max_mb':round(max(ys),2),'pss_slope_mb_per_min':round(60*sum((x-mx)*(y-my)for x,y in zip(xs,ys))/den,3)if den else 0}
thread=threading.Thread(target=sampler,daemon=True);thread.start()
try:
    app=wait(app_for_pid)
    click('Genel bakış');time.sleep(60)
    phase='idle_before';time.sleep(90)
    phase='navigation_stress'
    for i in range(80):
        for name in ['Odak zamanı','İstatistikler','Çalışma geçmişi','Derslerim','Genel bakış']:click(name);time.sleep(.06)
    phase='session_stress';click('Odak zamanı')
    for i in range(40):
        click('Odaklanmaya başla');wait(lambda:button('Duraklat'));click('Duraklat');wait(lambda:button('Devam et'));click('Devam et');wait(lambda:button('Bitir'));click('Bitir');wait(lambda:button('Odaklanmaya başla'))
    phase='idle_after';click('Genel bakış');time.sleep(180)
    phase='hidden_idle';click('Sistem tepsisine küçült');time.sleep(90)
    phase='closing'
    subprocess.run([str(binary)],env=env,timeout=15,stdout=subprocess.DEVNULL,stderr=subprocess.DEVNULL)
    click('Ayarlar');click('Kaydet ve uygulamadan çık');p.wait(timeout=15)
    result={'binary':str(binary),'temp_data':str(temp),'duration_seconds':round(time.monotonic()-start,1),'fixture_sessions':1000,'navigation_changes':400,'new_sessions':40,'idle_before':tail_stats('idle_before'),'idle_after':tail_stats('idle_after'),'hidden_idle':tail_stats('hidden_idle'),'inet_socket_observations':sorted(socket_observations),'exit_code':p.returncode}
    (out/'native-memory-summary.json').write_text(json.dumps(result,indent=2));print(json.dumps(result,indent=2))
finally:
    stop.set();thread.join(timeout=5)
    if p.poll() is None:
        p.terminate()
        try:p.wait(timeout=10)
        except subprocess.TimeoutExpired:p.kill()
    log.close()
