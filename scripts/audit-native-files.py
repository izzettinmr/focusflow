#!/usr/bin/env python3
"""Gerçek Tauri dosya diyalogları; yalnızca ayrı audit kimliği ve geçici veriler."""
from pathlib import Path
import os,time,tempfile,subprocess,json,sqlite3
import gi
gi.require_version('Atspi','2.0')
from gi.repository import Atspi, GLib
root=Path(__file__).resolve().parent.parent;out=root/'audit/2026-10-22';temp=Path(tempfile.mkdtemp(prefix='focusflow-dialog-audit-'))
binary=out/'focusflow-audit-fixed';db=temp/'data/com.focusflow.audit20261022/focusflow.db'
subprocess.run([str(root/'src-tauri/target/debug/examples/audit_probe'),'--seed',str(db),'3'],check=True)
env={**os.environ,'XDG_DATA_HOME':str(temp/'data'),'XDG_CONFIG_HOME':str(temp/'config'),'GTK_MODULES':'gail:atk-bridge','NO_AT_BRIDGE':'0','GDK_BACKEND':'x11'}

log=(out/'native-files-app.log').open('w');p=subprocess.Popen([str(binary)],env=env,stdout=log,stderr=log)
def wait(fn,timeout=30):
 end=time.monotonic()+timeout
 while time.monotonic()<end:
  if p.poll()is not None:raise RuntimeError('Test uygulaması kapandı')
  while GLib.MainContext.default().pending():GLib.MainContext.default().iteration(False)
  value=fn()
  if value:return value
  time.sleep(.15)
 raise TimeoutError('Beklenen test denetimi bulunamadı')
def walk(obj,depth=0):
 if depth>22:return
 try:obj.clear_cache()
 except Exception:pass
 yield obj
 try:
  for i in range(min(obj.get_child_count(),200)):
   child=obj.get_child_at_index(i)
   if child:yield from walk(child,depth+1)
 except Exception:pass
def app_pid():
 d=Atspi.get_desktop(0)
 for i in range(d.get_child_count()):
  a=d.get_child_at_index(i)
  try:
   if a.get_process_id()==p.pid:return a
  except Exception:pass
def find_button(obj,names):
 for o in walk(obj):
  try:
   if o.get_name()in names and o.get_role()==Atspi.Role.PUSH_BUTTON and o.get_state_set().contains(Atspi.StateType.SHOWING) and o.get_state_set().contains(Atspi.StateType.ENABLED) and o.get_action_iface()and o.get_action_iface().get_n_actions()>0:return o
  except Exception:pass
def click(names,obj=None):
 if isinstance(names,str):names=[names]
 b=wait(lambda:find_button(obj or app,names));assert b.get_action_iface().do_action(0);time.sleep(.15)
def dialog(title):
 d=Atspi.get_desktop(0)
 for i in range(d.get_child_count()):
  a=d.get_child_at_index(i)
  try:
   for j in range(a.get_child_count()):
    w=a.get_child_at_index(j)
    if title in (w.get_name()or''):return w
  except Exception:pass

def fill_filename(d,path,opened_location=False):
 candidates=[]
 for o in walk(d):
  try:
   edit=o.get_editable_text_iface()
   if edit and o.get_state_set().contains(Atspi.StateType.SHOWING):
    name=(o.get_name()or'').lower();parent=(o.get_parent().get_name()or'').lower();candidates.append((o,name,parent))
  except Exception:pass
 preferred=[o for o,n,parent in candidates if any(s in n+' '+parent for s in ['dosya adı','file name','filename'])]
 if not preferred:preferred=[o for o,n,_ in candidates if not any(s in n for s in ['search','ara'])]
 if len(preferred)!=1 and not opened_location:
  # GTK Open diyaloğunun konum alanı Ctrl+L ile açılır; yalnızca aktif test diyaloğu.
  import ctypes
  if not d.get_state_set().contains(Atspi.StateType.ACTIVE):raise RuntimeError('Test dosya seçicisi aktif değil; tuş gönderilmedi.')
  x=ctypes.CDLL('libX11.so.6');xt=ctypes.CDLL('libXtst.so.6');x.XOpenDisplay.restype=ctypes.c_void_p;x.XKeysymToKeycode.argtypes=[ctypes.c_void_p,ctypes.c_ulong];x.XKeysymToKeycode.restype=ctypes.c_ubyte;x.XFlush.argtypes=[ctypes.c_void_p];x.XCloseDisplay.argtypes=[ctypes.c_void_p];xt.XTestFakeKeyEvent.argtypes=[ctypes.c_void_p,ctypes.c_uint,ctypes.c_int,ctypes.c_ulong]
  display=x.XOpenDisplay(None)
  if not display:raise RuntimeError('Test X11 görüntüsü açılamadı')
  ctrl=x.XKeysymToKeycode(display,0xffe3);letter=x.XKeysymToKeycode(display,ord('l'))
  for key,pressed in [(ctrl,1),(letter,1),(letter,0),(ctrl,0)]:xt.XTestFakeKeyEvent(display,key,pressed,0)
  x.XFlush(display);x.XCloseDisplay(display);time.sleep(.4);return fill_filename(d,path,True)
 if len(preferred)!=1:
  names=[]
  for o in walk(d):
   try:
    if o.get_action_iface() and o.get_action_iface().get_n_actions()>0 and o.get_role() in [Atspi.Role.PUSH_BUTTON,Atspi.Role.TOGGLE_BUTTON]:names.append((o.get_name(),o.get_description()))
   except Exception:pass
  raise RuntimeError('Dosya adı alanı bulunamadı. Diyalog düğmeleri: '+str(names))
 assert preferred[0].get_editable_text_iface().set_text_contents(str(path));time.sleep(.3)
 assert Atspi.Text.get_text(preferred[0],0,-1)==str(path)

def choose(title,path,buttons):
 d=wait(lambda:dialog(title));fill_filename(d,path);click(buttons,d)

def count():
 with sqlite3.connect(db)as c:return c.execute('SELECT COUNT(*) FROM sessions').fetchone()[0]
try:
 app=wait(app_pid);click('Ayarlar')
 exported=temp/'exported.json';click('JSON yedeği oluştur');choose('FocusFlow · Dışa aktar',exported,['Kaydet','Save']);wait(lambda:exported.exists());saved=json.loads(exported.read_text());assert saved['version']==1 and len(saved['sessions'])==3
 assert exported.stat().st_mode&0o777==0o600
 print('Yerel JSON dışa aktarma ve 0600 izni geçti.',flush=True)
 click('Yedekten geri yükle');choose('FocusFlow · JSON yedeği seç',exported,['Aç','Open']);confirmation=wait(lambda:find_button(app,['Geri yükle']));assert confirmation.get_action_iface().do_action(0);wait(lambda:list(db.parent.glob('before-restore-*.json')));time.sleep(1);assert count()==3
 assert all(f.stat().st_mode&0o777==0o600 for f in db.parent.glob('before-restore-*.json'))
 print('Yerel onay diyaloğu, güvenlik yedeği ve geri yükleme geçti.',flush=True)
 malformed=temp/'malformed.json';malformed.write_text('{"version":1}');click('Yedekten geri yükle');choose('FocusFlow · JSON yedeği seç',malformed,['Aç','Open']);wait(lambda:find_button(app,['Tamam']));click('Tamam');assert count()==3
 click('JSON yedeği oluştur');d=wait(lambda:dialog('FocusFlow · Dışa aktar'));click(['Vazgeç','İptal','Cancel'],d)
 subprocess.run([str(binary)],env=env,timeout=15,stdout=subprocess.DEVNULL,stderr=subprocess.DEVNULL)
 click('Ayarlar');click('Kaydet ve uygulamadan çık');p.wait(timeout=15)
 result={'json_export':True,'private_mode_0600':True,'native_restore_confirmation':True,'automatic_safety_backup':True,'restore_same_count':True,'malformed_restore_preserved_data':True,'cancel_export':True,'exit_code':p.returncode,'temp_data':str(temp),'open_picker_fixture':'Gerçek GTK alanı ve aktif test penceresinde Ctrl+L'}
 (out/'native-files-summary.json').write_text(json.dumps(result,indent=2));print(json.dumps(result,indent=2))
except Exception as error:
 info={'error':repr(error),'exit_code':p.poll(),'own_alert':find_button(app,['Tamam'])is not None,'open_chooser_present':dialog('FocusFlow · JSON yedeği seç')is not None,'native_confirm_button':find_button(app,['Geri yükle'])is not None}
 print(json.dumps(info),flush=True)
 (out/'native-files-last-error.json').write_text(json.dumps(info,indent=2))
 raise
finally:
 if p.poll()is None:
  p.terminate()
  try:p.wait(timeout=10)
  except subprocess.TimeoutExpired:p.kill()
 log.close()
