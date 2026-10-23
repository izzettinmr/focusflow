#!/usr/bin/env python3
"""Start a named, headless test emulator; never selects a physical phone."""
from pathlib import Path
import os, subprocess, json, time
root=Path(__file__).resolve().parent.parent
tools=Path(os.environ.get('FOCUSFLOW_ANDROID_TOOLS',Path.home()/'.local/share/focusflow-android-tools'))
sdk=tools/'sdk'; env=os.environ.copy(); env['JAVA_HOME']=str(tools/'jdk'); env['ANDROID_HOME']=str(sdk)
name='FocusFlowAudit20261022'
avd=Path.home()/'.android/avd'/f'{name}.avd'
if not avd.exists():
    subprocess.run([str(sdk/'cmdline-tools/latest/bin/avdmanager'),'create','avd','--name',name,'--package','system-images;android-36;google_apis;x86_64','--device','pixel_5'],input='no\n',text=True,env=env,check=True)
out=root/'android/test-results'; out.mkdir(exist_ok=True)
with (out/'emulator.log').open('w') as log:
    p=subprocess.Popen([str(sdk/'emulator/emulator'),'-avd',name,'-port','5556','-no-window','-no-audio','-no-boot-anim','-no-snapshot','-gpu','swiftshader','-memory','2048','-cores','2','-camera-back','none','-camera-front','none','-no-metrics'],env=env,stdout=log,stderr=log,start_new_session=True)
time.sleep(1)
if p.poll() is not None: raise RuntimeError('Test emulator exited; see android/test-results/emulator.log')
(out/'emulator-process.json').write_text(json.dumps({'pid':p.pid,'avd':name,'serial':'emulator-5556'},indent=2))
print(f'Test emulator started: pid {p.pid}, serial emulator-5556')
