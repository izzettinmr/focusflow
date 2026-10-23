#!/usr/bin/env python3
"""Physical UI interactions in the named emulator, using the signed release APK."""
from pathlib import Path
import subprocess, time, json, xml.etree.ElementTree as ET
root=Path(__file__).resolve().parent.parent
out=root/'android/test-results'; out.mkdir(exist_ok=True)
serial='emulator-5556'; package='com.focusflow.mobile'
def adb(*args, binary=False):
    return subprocess.run(['adb','-s',serial,*args],check=True,stdout=subprocess.PIPE,stderr=subprocess.PIPE,timeout=90).stdout if binary else subprocess.run(['adb','-s',serial,*args],check=True,capture_output=True,text=True,timeout=90).stdout
assert 'FocusFlowAudit20261022' in adb('emu','avd','name')
print(adb('install','--no-streaming','-r',str(root/'android/output/FocusFlow.apk')),flush=True)
adb('shell','pm','grant',package,'android.permission.POST_NOTIFICATIONS')
adb('shell','input','keyevent','KEYCODE_WAKEUP'); adb('shell','wm','dismiss-keyguard')
print(adb('shell','am','start','-W','-n',package+'/com.focusflow.mobile.MainActivity'),flush=True)
def tap_node(n):
    import re
    x1,y1,x2,y2=map(int,re.findall(r'\d+',n.get('bounds','')))
    print('Touch',n.get('text'),n.get('bounds'),flush=True)
    adb('shell','input','tap',str((x1+x2)//2),str((y1+y2)//2))
def tree():
    adb('shell','uiautomator','dump','/sdcard/focusflow-release-ui.xml')
    raw=adb('exec-out','cat','/sdcard/focusflow-release-ui.xml')
    (out/'release-ui.xml').write_text(raw)
    nodes=list(ET.fromstring(raw).iter('node'))
    titles=[n.get('text','') for n in nodes]
    if any("FocusFlow" in t and 'responding' in t for t in titles): raise RuntimeError('FocusFlow ANR')
    if any(t.startswith(('System UI','Pixel Launcher')) and 'responding' in t for t in titles):
        close=next(n for n in nodes if n.get('text')=='Close app');tap_node(close);time.sleep(1);return tree()
    return nodes
def find(name, tap=False):
    for attempt in range(12):
        nodes=tree()
        navigation={'Genel bakış','Odak zamanı','İstatistikler','Çalışma geçmişi','Derslerim','Ayarlar'}
        nav_top=min([int(n.get('bounds').split('][')[0].split(',')[1]) for n in nodes if n.get('text') in navigation and n.get('clickable')=='true'] or [2340])
        for n in nodes:
            text=n.get('text','');desc=n.get('content-desc','')
            if name in [text,desc] and (not tap or n.get('clickable')=='true') and n.get('enabled')=='true':
                if n.get('bounds')=='[0,0][0,0]': continue
                if tap and name not in navigation:
                    import re
                    x1,y1,x2,y2=map(int,re.findall(r'\d+',n.get('bounds','')))
                    if (y1+y2)//2>=nav_top: continue
                if tap:tap_node(n)
                print(('Tapped ' if tap else 'Found ')+name,flush=True)
                return
        if tap and attempt in [2,5,8]: adb('shell','input','swipe','540','1850','540','800','350')
        time.sleep(.5)
    raise RuntimeError('Native release control unavailable: '+name)
find('Genel bakış')
find('Odak zamanı',True)
find('Odaklanmaya başla',True)
find('Duraklat')
time.sleep(2)
find('Duraklat',True)
find('Devam et')
find('Bitir',True)
find('Odaklanmaya başla')
(out/'release-phone.png').write_bytes(adb('exec-out','screencap','-p',binary=True))
package_info=adb('shell','dumpsys','package',package)
assert 'android.permission.INTERNET' not in package_info
assert not any('DEBUGGABLE' in line for line in package_info.splitlines() if 'flags=[' in line or 'pkgFlags=[' in line)
result={'release_installed':True,'actual_android_ui_start_pause_finish':True,'webview_debugging_disabled':True,'no_internet_permission':True}
(out/'release-smoke.json').write_text(json.dumps(result,indent=2)+'\n')
print('Signed release smoke passed',result,flush=True)
