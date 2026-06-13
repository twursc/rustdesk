import { FlutterAbility, FlutterEngine, MethodChannel, MethodCallHandler, MethodCall, MethodResult } from '@ohos/flutter_ohos';
import { GeneratedPluginRegistrant } from '../plugins/GeneratedPluginRegistrant';
import { common, bundleManager } from '@kit.AbilityKit';
import { hilog } from '@kit.PerformanceAnalysisKit';
import { fileIo as fs } from '@kit.CoreFileKit';
import { pasteboard } from '@kit.BasicServicesKit';
import { deviceInfo } from '@kit.BasicServicesKit';
import { settings } from '@kit.BasicServicesKit';
import { distributedDeviceManager } from '@kit.DistributedServiceKit';
import { abilityAccessCtrl, Permissions } from '@kit.AbilityKit';
import { window } from '@kit.ArkUI';

// 自检：dump 应用沙盒里 librustdesk.so 的真实物理路径与大小、bundle native lib 路径等。
// 日志 tag = rustdesk, domain = 0x3200。
function selfDiagnose(ctx: common.UIAbilityContext): void {
  hilog.info(0x3200, 'rustdesk', '=== rustdesk OHOS self-diagnose ===');
  hilog.info(0x3200, 'rustdesk', 'bundleCodeDir=%{public}s', ctx.bundleCodeDir);
  hilog.info(0x3200, 'rustdesk', 'filesDir=%{public}s', ctx.filesDir);
  hilog.info(0x3200, 'rustdesk', 'cacheDir=%{public}s', ctx.cacheDir);
  hilog.info(0x3200, 'rustdesk', 'tempDir=%{public}s', ctx.tempDir);

  const candidates: string[] = [
    ctx.bundleCodeDir + '/libs/arm64/librustdesk.so',
    ctx.bundleCodeDir + '/libs/arm64-v8a/librustdesk.so',
    '/data/storage/el1/bundle/libs/arm64/librustdesk.so',
    '/data/storage/el1/bundle/libs/arm64-v8a/librustdesk.so',
    '/data/storage/el1/bundle/entry/libs/arm64/librustdesk.so',
  ];
  for (const p of candidates) {
    try {
      const st = fs.statSync(p);
      hilog.info(0x3200, 'rustdesk', 'FOUND %{public}s size=%{public}d', p, st.size);
    } catch (e) {
      hilog.warn(0x3200, 'rustdesk', 'miss  %{public}s err=%{public}s', p, JSON.stringify(e));
    }
  }

  // libs 目录下所有 .so 列表
  const libDirs: string[] = [
    ctx.bundleCodeDir + '/libs/arm64',
    ctx.bundleCodeDir + '/libs/arm64-v8a',
    '/data/storage/el1/bundle/libs/arm64',
  ];
  for (const d of libDirs) {
    try {
      const names = fs.listFileSync(d);
      hilog.info(0x3200, 'rustdesk', 'dir %{public}s -> %{public}s', d, names.join(','));
    } catch (e) {
      hilog.warn(0x3200, 'rustdesk', 'dir miss %{public}s', d);
    }
  }

  // bundle 信息
  try {
    const flags = bundleManager.BundleFlag.GET_BUNDLE_INFO_WITH_HAP_MODULE
      | bundleManager.BundleFlag.GET_BUNDLE_INFO_WITH_APPLICATION;
    const info = bundleManager.getBundleInfoForSelfSync(flags);
    hilog.info(0x3200, 'rustdesk', 'bundleName=%{public}s', info.name);
    hilog.info(0x3200, 'rustdesk', 'app.nativeLibraryPath=%{public}s',
      info.appInfo.nativeLibraryPath ?? '<undef>');
    for (const m of info.hapModulesInfo) {
      hilog.info(0x3200, 'rustdesk',
        'module=%{public}s nativeLibraryPath=%{public}s',
        m.name, m.nativeLibraryPath ?? '<undef>');
    }
  } catch (e) {
    hilog.error(0x3200, 'rustdesk', 'getBundleInfoForSelfSync fail %{public}s', JSON.stringify(e));
  }
}

// RustDesk 主控端 v0.1: 实现 dart 端 mChannel MethodChannel 的最小子集，
// 兼容 Android 端 mChannel 接口。v0.2 加入被控端时再补 start_capture / stop_service 等。
class MainChannel implements MethodCallHandler {
  private context: common.UIAbilityContext;

  constructor(context: common.UIAbilityContext) {
    this.context = context;
  }

  onMethodCall(call: MethodCall, result: MethodResult): void {
    // 注意：dart Map<String,dynamic> 传到 ArkTS 端 args 是 Map<Any,Any>，
    // 不是 plain JS object，所以 JSON.stringify(call.args) 永远是 '{}'，
    // 用 `(call.args as Record<string,string>)['key']` 也拿不到。必须用
    // call.argument(key)（@ohos/flutter_ohos MethodCall.ets 提供的方法，
    // 内部判断 Map / Object 两种类型）。
    hilog.info(0x3200, 'rustdesk', 'mChannel >> %{public}s', call.method);

    switch (call.method) {
      // dart 端调试上报通道：dart side 拿不到 hilog，借这个把字符串日志带到 ArkTS。
      // dart 用法：await mChannel.invokeMethod('rustdesk_debug_log', {'msg': '...'});
      case 'rustdesk_debug_log': {
        const msg = (call.argument('msg') as string) ?? '<no msg>';
        hilog.info(0x3200, 'rustdesk', 'DART >> %{public}s', msg);
        result.success(true);
        break;
      }
      case 'init_service':
      case 'check_service':
      case 'sync_app_dir':
        // app data 目录由 path_provider 单独返回，这里只确认服务初始化即可
        result.success(this.context.filesDir);
        break;
      case 'get_value': {
        const key = (call.argument('name') as string) ?? '';
        result.success(this.getValue(key));
        break;
      }
      // 返回本机设备名 + 稳定 id 给 dart，让它调 mainDeviceName/mainDeviceId
      // 写到 rust DEVICE_NAME/DEVICE_ID，对端连过来就能看见真实设备名（之前
      // device_info_plus 在 OHOS 没实现，显示 'Flutter'）。
      // name 取舍：系统设置 → "关于本机" 里用户改的"设备名称"优先，没设
      // 就回退到 marketName（如 "Mate 60 Pro"），最后回退 brand-productModel。
      // id 用 brand+productModel 的简单稳定 hash，不上 udid（要 system_basic 权限）。
      case 'get_device_info': {
        const fallbackName =
          (deviceInfo.marketName && deviceInfo.marketName.length > 0)
            ? deviceInfo.marketName
            : `${deviceInfo.brand}-${deviceInfo.productModel}`;
        const stableId = `${deviceInfo.brand}_${deviceInfo.productModel}`;

        // 优先用 distributedDeviceManager.getLocalDeviceName()，跟系统"设置→
        // 关于本机→设备名称"里用户自定义的那个一致（蓝牙/热点也是用这个）。
        // 需要 ohos.permission.DISTRIBUTED_DATASYNC（首次启动已申请）。失败/
        // 没权限时落到 settings 路径，再 fallback 到 marketName。
        let resolved = false;
        try {
          const dm = distributedDeviceManager.createDeviceManager(
            this.context.applicationInfo.name);
          const name = dm.getLocalDeviceName();
          if (name && name.length > 0) {
            hilog.info(0x3200, 'rustdesk',
              'get_device_info: distributedDeviceManager name=%{public}s', name);
            result.success({ 'name': name, 'id': stableId });
            resolved = true;
          }
        } catch (e) {
          hilog.warn(0x3200, 'rustdesk',
            'get_device_info: distributedDeviceManager throw %{public}s',
            JSON.stringify(e));
        }
        if (!resolved) {
          settings.getValue(this.context, settings.general.DEVICE_NAME)
            .then((v: string) => {
              const name = (v && v.length > 0) ? v : fallbackName;
              result.success({ 'name': name, 'id': stableId });
            })
            .catch(() => {
              result.success({ 'name': fallbackName, 'id': stableId });
            });
        }
        break;
      }
      case 'enable_soft_keyboard': {
        // 软键盘控制：v0.1 直接成功；具体由 dart 端 FocusNode 处理
        result.success(true);
        break;
      }
      case 'on_voice_call_started':
      case 'on_voice_call_closed':
      case 'cancel_notification':
      case 'stop_input':
      case 'check_permission':
      case 'request_permission':
      case 'check_video_permission':
      case 'start_action':
        // 控制端不需要这些（被控端能力，v0.2 实现）；直接 ack。
        result.success(true);
        break;
      // 本机→远端：读 OHOS 系统剪贴板纯文本返回给 dart，让它走
      // bind.session_input_string 推到对端。dart 是 fire-and-forget 但
      // 我们仍然给 success 一个字符串作为结果。
      case 'try_sync_clipboard': {
        const sys = pasteboard.getSystemPasteboard();
        sys.getData((err, data) => {
          if (err || !data) {
            hilog.warn(0x3200, 'rustdesk',
              'try_sync_clipboard: pasteboard empty/err=%{public}s',
              err ? JSON.stringify(err) : 'null');
            result.success('');
            return;
          }
          let text = '';
          try {
            text = data.getPrimaryText() ?? '';
          } catch (e) {
            hilog.warn(0x3200, 'rustdesk',
              'try_sync_clipboard: getPrimaryText fail %{public}s', JSON.stringify(e));
          }
          result.success(text);
        });
        break;
      }
      // 远端→本机：把 rust 推过来的远端剪贴板文字写到 OHOS 系统剪贴板。
      // dart 用法：mChannel.invokeMethod('set_clipboard_text', {'text': '...'})
      case 'set_clipboard_text': {
        const text = (call.argument('text') as string) ?? '';
        try {
          const sys = pasteboard.getSystemPasteboard();
          const record = pasteboard.createData(pasteboard.MIMETYPE_TEXT_PLAIN, text);
          sys.setData(record, (err) => {
            if (err) {
              hilog.warn(0x3200, 'rustdesk',
                'set_clipboard_text: setData err %{public}s', JSON.stringify(err));
            }
            result.success(true);
          });
        } catch (e) {
          hilog.warn(0x3200, 'rustdesk',
            'set_clipboard_text: throw %{public}s', JSON.stringify(e));
          result.success(false);
        }
        break;
      }
      // 被控端方法 stub
      case 'start_capture':
      case 'stop_service':
      case 'get_start_on_boot_opt':
      case 'set_start_on_boot_opt':
        result.success(false);
        break;
      default:
        hilog.warn(0x3200, 'rustdesk', 'mChannel: unknown method %{public}s', call.method);
        result.notImplemented();
    }
  }

  private getValue(key: string): string {
    switch (key) {
      case 'screen_size':
        // TODO: 真实从 display API 取分辨率
        return JSON.stringify({ width: 1080, height: 1920, scale: 1 });
      case 'app_dir':
        return this.context.filesDir;
      case 'is_start':
        return 'false';
      default:
        return '';
    }
  }
}

// 首次启动主动申请 DISTRIBUTED_DATASYNC（用于拿系统设备名）。已授权则立即返回。
// dart 在 PlatformFFI.init 里调 get_device_info 之前发生：configureFlutterEngine
// 完成后立刻并发触发，竞争上没保证 — 但用户拒/同意都不影响主流程，下次启动
// 已记住的授权也会让 getLocalDeviceName 工作。
async function requestDeviceNamePermissionIfNeeded(ctx: common.UIAbilityContext): Promise<void> {
  const perm: Permissions = 'ohos.permission.DISTRIBUTED_DATASYNC';
  const atm = abilityAccessCtrl.createAtManager();
  try {
    const granted = await atm.checkAccessToken(
      bundleManager.getBundleInfoForSelfSync(0).appInfo.accessTokenId, perm);
    if (granted === abilityAccessCtrl.GrantStatus.PERMISSION_GRANTED) {
      hilog.info(0x3200, 'rustdesk', 'DISTRIBUTED_DATASYNC already granted');
      return;
    }
    const res = await atm.requestPermissionsFromUser(ctx, [perm]);
    hilog.info(0x3200, 'rustdesk',
      'DISTRIBUTED_DATASYNC request result: %{public}s', JSON.stringify(res.authResults));
  } catch (e) {
    hilog.warn(0x3200, 'rustdesk',
      'request DISTRIBUTED_DATASYNC throw %{public}s', JSON.stringify(e));
  }
}

export default class EntryAbility extends FlutterAbility {
  private mChannel: MethodChannel | null = null;

  configureFlutterEngine(flutterEngine: FlutterEngine): void {
    super.configureFlutterEngine(flutterEngine);
    GeneratedPluginRegistrant.registerWith(flutterEngine);

    selfDiagnose(this.context);

    // mChannel 与 Android 同名，dart 端 NativeModel._toAndroidChannel 直接复用
    this.mChannel = new MethodChannel(
      flutterEngine.dartExecutor.getBinaryMessenger(),
      'mChannel'
    );
    this.mChannel.setMethodCallHandler(new MainChannel(this.context));
    hilog.info(0x3200, 'rustdesk', 'mChannel handler registered (configureFlutterEngine)');
  }

  // 权限框必须在 ability 进入前台、UI 可见的阶段才能弹出来。
  // configureFlutterEngine 阶段 ability 还在创建，window stage 没起来，弹窗
  // 会被静默吞掉。这里 override onWindowStageCreate 在窗口真正可见后再请求。
  onWindowStageCreate(windowStage: window.WindowStage): void {
    super.onWindowStageCreate(windowStage);
    requestDeviceNamePermissionIfNeeded(this.context);
  }
}
