prefs-vp-other = 其他
prefs-vp-plugins = 插件
prefs-vp-intro-skipper =
    .title = 介绍跳过插件
    .subtitle =
        在剧集介绍期间显示跳过介绍按钮。
        这需要在您的服务器上安装<a href="{ $introSkipperUrl }">介绍跳过</a>插件。
prefs-skip-amount =
    { $seconds } { $seconds ->
        [one] 秒
       *[other] 秒
    }
prefs-vp-skip-backwards =
    .title = 向后跳过的量
    .subtitle = 一次向后跳过多少秒
prefs-vp-skip-forwards =
    .title = 向前跳过的量
    .subtitle = 一次向前跳过多少秒
prefs-window-title = 首选项
prefs-general-page = 常规
prefs-vp-page = 视频播放器
prefs-general-language =
    .title = 语言
    .subtitle = 为翻译做出贡献 <a href="{ $weblateUrl }">Weblate</a>
    .option-default = 系统默认 ({ $languageId })
prefs-general-theme =
    .title = 主题
    .option-default = 系统默认
    .option-light = 浅色
    .option-dark = 深色
prefs-general-restore-most-recent =
    .title = { app-name } 启动时自动登录
    .subtitle = 启动时，您将登录到最近使用的账号
prefs-vp-interface = 界面
prefs-general-use-episode-image =
    .title = 在“下一集”和“继续观看”部分使用剧集图片
    .subtitle = 下一集和继续观看部分将使用剧集图片作为缩略图，而不是节目的主缩略图。
prefs-vp-on-left-click =
    .title = 单击鼠标时
    .subtitle = 当您在视频播放器中左键单击时会发生什么
prefs-vp-on-left-click-options =
    .play-pause = 播放/暂停视频
    .toggle-controls = 显示/隐藏控件
prefs-vp-subs = 字幕
prefs-vp-subs-reset =
    .label = 重置
    .tooltip = 重置字幕设置
prefs-vp-subs-style = 字幕风格
prefs-vp-subs-style-ass-warning =
    这些设置不适用于 ASS/SSA 字幕。
    有关 ASS/SSA 字幕，请参阅“{ prefs-vp-subs-more }”。
prefs-vp-subs-colour =
    .title = 字幕文本颜色
prefs-vp-subs-background-colour =
    .title = 字幕背景颜色
prefs-vp-subs-more = 更多字幕首选项
prefs-vp-subs-scale =
    .title = 字幕比例
    .subtitle = 字幕文本的比例系数
prefs-vp-subs-position =
    .title = 字幕位置
    .subtitle = 其中 0 是屏幕的顶部，100 是底部
prefs-vp-intro-skipper-auto-skip =
    .title = 自动跳过介绍
    .subtitle = 跳过介绍，无需按下跳过介绍按钮
prefs-vp-jellyscrub =
    .title = Jellyscrub 插件
    .subtitle =
        将鼠标悬停在视频进度条上时显示缩略图。
        这需要在您的服务器上安装 <a href="{ $jellyscrubUrl }">Jellyscrub</a> 插件。
        在支持的服务器上将改用 Jellyfin 的原生特技播放。
prefs-vp-experimental =
    .title = 实验性首选项
    .subtitle =
        这些首选项可能已损坏或不完整。
        不建议修改它们。
prefs-vp-backend =
    .title = 视频播放器后端
    .subtitle = 需要重启 { app-name }.
    .value-mpv = MPV
    .value-gstreamer = 媒体框架
prefs-vp-subs-font =
    .title = 字幕字体
    .subtitle = 仅列出支持的字体
prefs-vp-hls-playback =
    .title = HLS 播放
    .subtitle = 这可能会中断音频和字幕轨道选择。
prefs-vp-subs-more-ass-warning =
    .title = 这些首选项将影响 ASS/SSA 字幕！
    .subtitle =
        ASS/SSA 字幕通常包含特殊风格和位置。更改这些首选项可能会导致 ASS/SSA 字幕渲染不正确。

        如果您只想更改其他字幕格式的风格，建议您更改“{ prefs-vp-subs-style }”下的首选项。
prefs-vp-mpv-hwdec =
    .title = 硬件解码
    .subtitle =
        硬件解码可以提高播放性能。
        如果遇到问题，请选择“{ prefs-vp-mpv-hwdec-options.auto-safe }”或“{ prefs-vp-mpv-hwdec-options.off }”。
        有关更多信息，请参阅 <a href="{ $mpvManualUrl }">MPV 手册</a>。
prefs-vp-mpv-hwdec-options =
    .auto = Auto
    .auto-copy = Auto (copy)
    .auto-safe = Auto (safe)
    .custom = 自定义
    .off = 关
