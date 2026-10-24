#!/usr/bin/env python3
"""Small, reproducible Android SDK build without Gradle or Android Studio.

The same Rust/SQLite engine is embedded via JNI. Java has no external libraries.
Signing material remains in a user-private directory, outside the source tree.
"""
from pathlib import Path
import argparse, hashlib, json, os, secrets, shutil, subprocess, zipfile

root = Path(__file__).resolve().parent.parent
parser = argparse.ArgumentParser()
parser.add_argument('--audit', action='store_true', help='Separate debug package for an emulator')
parser.add_argument('--abi', choices=['arm64-v8a', 'x86_64', 'all'], default='all')
args = parser.parse_args()
tools = Path(os.environ.get('FOCUSFLOW_ANDROID_TOOLS', Path.home()/'.local/share/focusflow-android-tools'))
sdk = Path(os.environ.get('ANDROID_HOME', tools/'sdk'))
jdk = Path(os.environ.get('JAVA_HOME', tools/'jdk'))
ndk = Path(os.environ.get('NDK_HOME', sdk/'ndk/29.0.14206865'))
bt = sdk/'build-tools/36.0.0'
android = sdk/'platforms/android-36/android.jar'
for f in [jdk/'bin/javac', bt/'aapt2', android, ndk/'source.properties']:
    if not f.exists(): raise SystemExit(f'Missing {f}. Run bash scripts/setup-android.sh first.')
env = os.environ.copy()
env['JAVA_HOME'] = str(jdk)
env['PATH'] = str(jdk/'bin')+':'+str(Path.home()/'.cargo/bin')+':'+env['PATH']
build = root/'android/build'/('audit' if args.audit else 'release')
build.mkdir(parents=True, exist_ok=True)
def run(*command, **kwargs): subprocess.run([str(v) for v in command], cwd=root, env=env, check=True, **kwargs)
run('npm','run','build')
web = build/'assets/web'
if web.exists(): shutil.rmtree(web)
shutil.copytree(root/'dist',web)
shutil.copyfile(root/'src/locales/messages.en.json', build/'assets/messages.en.json')
abis = {'arm64-v8a': ('aarch64-linux-android','aarch64-linux-android26-clang'), 'x86_64': ('x86_64-linux-android','x86_64-linux-android26-clang')}
if args.abi!='all': abis={args.abi: abis[args.abi]}
for abi,(target,clang) in abis.items():
    native_env=env.copy()
    toolbin=ndk/'toolchains/llvm/prebuilt/linux-x86_64/bin'
    native_env['CC_'+target.replace('-','_')]=str(toolbin/clang)
    native_env['AR_'+target.replace('-','_')]=str(toolbin/'llvm-ar')
    native_env['CARGO_TARGET_'+target.upper().replace('-','_')+'_LINKER']=str(toolbin/clang)
    native_env['RUSTFLAGS']='-C link-arg=-Wl,-z,max-page-size=16384'
    subprocess.run(['cargo','build','--locked','--release','--manifest-path','android/rust/Cargo.toml','--target',target],cwd=root,env=native_env,check=True)
res=build/'res'
if res.exists(): shutil.rmtree(res)
shutil.copytree(root/'android/res',res)
(res/'raw').mkdir(exist_ok=True)
shutil.copyfile(root/'assets/sounds/start.wav',res/'raw/focusflow_start.wav')
shutil.copyfile(root/'assets/sounds/complete.wav',res/'raw/focusflow_complete.wav')
shutil.copy2(root/'src-tauri/icons/128x128.png',res/'drawable/icon.png')
manifest=(root/'android/AndroidManifest.xml').read_text()
if args.audit: manifest=manifest.replace('<application ', '<application android:debuggable="true" ')
manifest_file=build/'AndroidManifest.xml'; manifest_file.write_text(manifest)
generated=build/'generated'; classes=build/'classes'; dex=build/'dex'
for p in [generated,classes,dex]:
    if p.exists(): shutil.rmtree(p)
    p.mkdir()
run(bt/'aapt2','compile','--dir',res,'-o',build/'resources.zip')
cmd=[bt/'aapt2','link','-o',build/'unsigned.apk','--manifest',manifest_file,'-I',android,'--java',generated,'-A',build/'assets',build/'resources.zip']
if args.audit: cmd+=['--rename-manifest-package','com.focusflow.mobile.audit','--custom-package','com.focusflow.mobile']
run(*cmd)
sources=list((root/'android/java').rglob('*.java'))+list(generated.rglob('*.java'))
run(jdk/'bin/javac','-encoding','UTF-8','-source','8','-target','8','-bootclasspath',str(android)+os.pathsep+str(bt/'core-lambda-stubs.jar'),'-d',classes,*sources)
run(jdk/'bin/jar','cf',build/'classes.jar','-C',classes,'.')
run(bt/'d8','--min-api','26','--lib',android,'--output',dex,build/'classes.jar')
with zipfile.ZipFile(build/'unsigned.apk','a') as z:
    for p in dex.glob('*.dex'): z.write(p,p.name)
    for abi,(target,_) in abis.items():
        z.write(root/f'android/rust/target/{target}/release/libfocusflow_android.so',f'lib/{abi}/libfocusflow_android.so',compress_type=zipfile.ZIP_STORED)
run(bt/'zipalign','-P','16','-f','4',build/'unsigned.apk',build/'aligned.apk')
signing=Path.home()/'.local/share/focusflow-android-signing'
signing.mkdir(mode=0o700,parents=True,exist_ok=True); signing.chmod(0o700)
key=signing/'release.p12'; password=signing/'password'
if not key.exists():
    if password.exists(): raise SystemExit('Signing password exists without keystore; inspect signing directory before continuing.')
    with open(password, 'x', opener=lambda p,flags: os.open(p,flags,0o600)) as f: f.write(secrets.token_urlsafe(32))
    run(jdk/'bin/keytool','-genkeypair','-keystore',key,'-storetype','PKCS12','-storepass:file',password,'-alias','focusflow','-keyalg','RSA','-keysize','3072','-validity','10000','-dname','CN=FocusFlow Local, O=FocusFlow, C=TR')
    key.chmod(0o600)
output=root/'android/output'; output.mkdir(exist_ok=True)
apk=output/('FocusFlow-audit.apk' if args.audit else 'FocusFlow.apk')
run(bt/'apksigner','sign','--ks',key,'--ks-pass','file:'+str(password),'--out',apk,build/'aligned.apk')
run(bt/'apksigner','verify','--verbose',apk)
run(bt/'zipalign','-c','-P','16','4',apk)
info={'apk':apk.name,'sha256':hashlib.sha256(apk.read_bytes()).hexdigest(),'bytes':apk.stat().st_size,'abis':list(abis),'minimum_android':'8.0 (API 26)','target_sdk':36,'audit':args.audit}
apk.with_suffix('.json').write_text(json.dumps(info,indent=2)+'\n')
print(json.dumps(info,indent=2)); print(f'APK ready: {apk}')
