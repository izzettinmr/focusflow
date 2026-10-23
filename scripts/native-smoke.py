#!/usr/bin/env python3
"""Gerçek Tauri/WebKit penceresi + SQLite. Linux AT-SPI ve aktif masaüstü gerekir.
Kişisel veri dizini yerine geçici XDG dizini kullanır; test penceresini açar.
"""
import os, sys, time, tempfile, subprocess, sqlite3, json
from pathlib import Path
import gi
gi.require_version('Atspi', '2.0')
from gi.repository import Atspi, Gio, GLib
root=Path(__file__).resolve().parent.parent
binary=Path(os.environ.get('FOCUSFLOW_NATIVE_BINARY',str(root/'src-tauri/target/release/focusflow')))
temp=Path(tempfile.mkdtemp(prefix='focusflow-native-'))
env={**os.environ,'XDG_DATA_HOME':str(temp/'data'),'XDG_CONFIG_HOME':str(temp/'config'),'NO_AT_BRIDGE':'0','GTK_MODULES':'gail:atk-bridge'}
log=open(temp/'app.log','w')
notification_log=open(temp/'notifications.log','w')
monitor=None
if '--notification' in sys.argv:
    monitor=subprocess.Popen(['dbus-monitor','--session',"type='method_call',interface='org.freedesktop.Notifications',member='Notify',arg3='FocusFlow'"],stdout=notification_log,stderr=subprocess.DEVNULL)
if '--notification' in sys.argv:
    # Gerçek SQLite ayarını test başlamadan önce 1 dakikaya kur; süreyi hızlandırma.
    seed_db=temp/'data/com.focusflow.desktop/focusflow.db'
    seed_db.parent.mkdir(parents=True)
    bridge=root/'src-tauri/target/debug/examples/ui_bridge'
    payload={'cmd':'save_settings','args':{'settings':{'work_minutes':1,'short_minutes':5,'long_minutes':15,'goal_minutes':240,'long_every':4}}}
    seeded=subprocess.run([str(bridge),str(seed_db)],input=json.dumps(payload)+'\n',text=True,capture_output=True,check=True)
    assert '"error"' not in seeded.stdout,seeded.stdout
proc=subprocess.Popen([str(binary)],env=env,stdout=log,stderr=log)

def wait(fn, timeout=30):
    until=time.monotonic()+timeout
    while time.monotonic()<until:
        result=fn()
        if result:return result
        if proc.poll() is not None:raise RuntimeError((temp/'app.log').read_text())
        time.sleep(.2)
    raise TimeoutError('Yerel pencere/denetim bulunamadı')

def find_app():
    desktop=Atspi.get_desktop(0)
    for i in range(desktop.get_child_count()):
        child=desktop.get_child_at_index(i)
        try:
            if child.get_process_id()==proc.pid:return child
        except Exception:pass

def walk(obj, depth=0):
    if depth>22:return
    yield obj
    try:
        for i in range(min(obj.get_child_count(),200)):
            c=obj.get_child_at_index(i)
            if c:yield from walk(c,depth+1)
    except Exception:pass

def control(name):
    for o in walk(app):
        try:
            if o.get_name()==name and o.get_action_iface() and o.get_action_iface().get_n_actions()>0:return o
        except Exception:pass

def click(name):
    o=wait(lambda:control(name))
    assert o.get_action_iface().do_action(0),name

try:
    app=wait(find_app)
    click('Odaklanmaya başla')
    db=wait(lambda:next((temp/'data').rglob('focusflow.db'),None))
    def rows():
        with sqlite3.connect(db) as c:return c.execute('SELECT status,elapsed_ms FROM sessions').fetchall()
    wait(lambda:rows() and rows()[0][0]=='running')
    time.sleep(2.3)
    click('Duraklat')
    wait(lambda:rows()[0][0]=='paused')
    paused=rows()[0][1];assert paused>=1000
    time.sleep(1.3);assert rows()[0][1]==paused
    click('Devam et');time.sleep(1.3);click('Bitir')
    wait(lambda:rows()[0][0]=='early')
    assert rows()[0][1]>paused
    # KDE/StatusNotifier tepsisine bu sürecin gerçekten kayıt olduğunu doğrula.
    bus=Gio.bus_get_sync(Gio.BusType.SESSION,None)
    result=bus.call_sync('org.kde.StatusNotifierWatcher','/StatusNotifierWatcher','org.freedesktop.DBus.Properties','Get',GLib.Variant('(ss)',('org.kde.StatusNotifierWatcher','RegisteredStatusNotifierItems')),None,Gio.DBusCallFlags.NONE,5000,None)
    items=result.unpack()[0]
    registered=False
    for item in items:
        service=item.split('/')[0]
        try:
            pid=bus.call_sync('org.freedesktop.DBus','/org/freedesktop/DBus','org.freedesktop.DBus','GetConnectionUnixProcessID',GLib.Variant('(s)',(service,)),None,Gio.DBusCallFlags.NONE,5000,None).unpack()[0]
            if pid==proc.pid:registered=True
        except Exception:pass
    assert registered,'Sistem tepsisi kaydı bulunamadı'
    click('Sistem tepsisine küçült')
    reopened=subprocess.run([str(binary)],env=env,timeout=15,stdout=subprocess.DEVNULL,stderr=subprocess.DEVNULL)
    assert reopened.returncode==0
    if '--notification' in sys.argv:
        click('Odaklanmaya başla')
        with sqlite3.connect(db) as c:
            target=c.execute('SELECT target_ms FROM sessions ORDER BY id DESC LIMIT 1').fetchone()[0]
        assert target==60000,('Yerel sayı alanı güncellenemedi',target)
        click('Sistem tepsisine küçült')
        wait(lambda:rows()[-1][0]=='completed',timeout=75)
        wait(lambda:'Pomodoro tamamlandı' in (temp/'notifications.log').read_text(),timeout=15)
        reopened=subprocess.run([str(binary)],env=env,timeout=15,stdout=subprocess.DEVNULL,stderr=subprocess.DEVNULL)
        assert reopened.returncode==0
    click('Ayarlar');click('Kaydet ve uygulamadan çık')
    proc.wait(timeout=15)
    assert proc.returncode==0
    print('Yerel masaüstü testi geçti: Tauri/WebKit IPC, SQLite, duraklat/devam/bitir, tepsi kaydı, gizle/aç, tek uygulama örneği ve temiz çıkış.')
    if '--notification' in sys.argv:print('1 dakikalık gerçek Pomodoro tamamlandı ve masaüstü bildirim çağrısı doğrulandı.')
    print('Geçici kayıtlar:',temp)
finally:
    if proc.poll() is None:
        proc.terminate()
        try:proc.wait(timeout=5)
        except subprocess.TimeoutExpired:proc.kill()
    if monitor is not None:monitor.terminate();monitor.wait(timeout=5)
    notification_log.close()
    log.close()
