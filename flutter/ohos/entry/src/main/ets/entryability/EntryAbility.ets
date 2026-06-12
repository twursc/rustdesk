import { FlutterAbility, FlutterEngine, MethodChannel, MethodCallHandler, MethodCall, MethodResult } from '@ohos/flutter_ohos';
import { GeneratedPluginRegistrant } from '../plugins/GeneratedPluginRegistrant';
import { common } from '@kit.AbilityKit';
import { hilog } from '@kit.PerformanceAnalysisKit';

// RustDesk 主控端 v0.1: 实现 dart 端 mChannel MethodChannel 的最小子集，
// 兼容 Android 端 mChannel 接口。v0.2 加入被控端时再补 start_capture / stop_service 等。
class MainChannel implements MethodCallHandler {
  private context: common.UIAbilityContext;

  constructor(context: common.UIAbilityContext) {
    this.context = context;
  }

  onMethodCall(call: MethodCall, result: MethodResult): void {
    hilog.info(0x3200, 'rustdesk', 'mChannel: %{public}s', call.method);
    switch (call.method) {
      case 'init_service':
      case 'check_service':
      case 'sync_app_dir':
        // app data 目录由 path_provider 单独返回，这里只确认服务初始化即可
        result.success(this.context.filesDir);
        break;
      case 'get_value': {
        const key = call.argument<string>('name') ?? '';
        result.success(this.getValue(key));
        break;
      }
      case 'enable_soft_keyboard': {
        // 软键盘控制：v0.1 直接成功；具体由 dart 端 FocusNode 处理
        result.success(true);
        break;
      }
      case 'on_voice_call_started':
      case 'on_voice_call_closed':
      case 'try_sync_clipboard':
      case 'cancel_notification':
      case 'stop_input':
      case 'check_permission':
      case 'request_permission':
      case 'check_video_permission':
      case 'start_action':
        // 控制端不需要这些（被控端能力，v0.2 实现）；直接 ack。
        result.success(true);
        break;
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

export default class EntryAbility extends FlutterAbility {
  private mChannel: MethodChannel | null = null;

  configureFlutterEngine(flutterEngine: FlutterEngine): void {
    super.configureFlutterEngine(flutterEngine);
    GeneratedPluginRegistrant.registerWith(flutterEngine);

    // mChannel 与 Android 同名，dart 端 NativeModel._toAndroidChannel 直接复用
    this.mChannel = new MethodChannel(
      flutterEngine.dartExecutor.getBinaryMessenger(),
      'mChannel'
    );
    this.mChannel.setMethodCallHandler(new MainChannel(this.context));
  }
}
