import 'dart:io';

// OHOS：dart:io 当前会把 ohos 报为 'ohos' Platform.operatingSystem。
final isOhos_ = Platform.operatingSystem == 'ohos';
final isAndroid_ = Platform.isAndroid;
final isIOS_ = Platform.isIOS;
final isWindows_ = Platform.isWindows && !isOhos_;
final isMacOS_ = Platform.isMacOS;
final isLinux_ = Platform.isLinux && !isOhos_;
final isWeb_ = false;
final isWebDesktop_ = false;

// isDesktop 不包含 OHOS（OHOS 是手机/平板，走 mobile UI）
final isDesktop_ = isWindows_ || isMacOS_ || isLinux_;

String get screenInfo_ => '';

final isWebOnWindows_ = false;
final isWebOnLinux_ = false;
final isWebOnMacOS_ = false;
