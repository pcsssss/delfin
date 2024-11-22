vp-unnamed-item = 未命名项目
vp-next-up-action =
    .play = 立即播放
    .hide = 隐藏
vp-fullscreen-tooltip =
    { $enter ->
        [true] 进入
       *[false] 退出
    } 全屏
vp-next-prev-episode-tooltip =
    { $direction ->
        [next] 下一
       *[previous] 上一
    } 集
vp-play-pause-tooltip =
    { $playing ->
        [true] 暂停
       *[false] 播放
    }
vp-skip-forwards-backwards-tooltip =
    跳过 { $direction ->
        [forwards] 前进
       *[backwards] 后退
    } { $seconds } { $seconds ->
        [one] 秒
       *[other] 秒
    }
vp-skip-intro =
    .manual = 跳过介绍
    .auto = 跳过介绍 { $seconds }…
vp-audio-track-tooltip = 音轨
vp-audio-track-menu = 音轨
vp-subtitle-track-menu = 字幕轨道
vp-subtitle-track-external = 外部字幕轨道
vp-subtitle-track-off = 关
vp-no-subtitles-available = 无字幕。
vp-volume-mute-tooltip =
    { $muted ->
        [true] 取消静音
       *[false] 静音
    }
vp-playback-speed-tooltip = 播放速度
vp-playback-speed-normal = 正常
vp-unnamed-track = 未命名轨道
vp-next-up-starting =
    下一集开始从 { $remaining } { $remaining ->
        [one] 秒
       *[other] 秒
    }…
vp-duration-tooltip =
    .total = 切换到剩余时间
    .remaining = 转换到总持续时间
vp-backend-mpv-track-name =
    .title-and-language = { $title } – { $language }
    .id-and-language = 轨道 { $id } – { $language }
    .id = 轨道 { $id }
vp-subtitle-track-tooltip =
    { $subtitlesAvailable ->
        [true] 字幕轨道
       *[false] 没有可用的字幕轨道
    }
vp-playback-speed-toast = 播放速度：{ $speed }x
vp-backend-gst-track-name = { $displayName } – { $title }
vp-menu-debug-copy-stream-url = 复制流 URL
vp-menu-debug-copy-stream-url-toast =
    .success = 流 URL 已复制到剪贴板
    .fail = 无法将流 URL 复制到剪贴板
vp-menu-debug-mpv-props = MPV 属性
vp-menu-hwdec = 硬件解码
vp-menu-debug = 调试
vp-menu-deinterlace-options =
    .on = 开
    .off = 关
    .auto = 自动
vp-menu-deinterlace = 反交错
vp-menu-deinterlace-toast =
    .on = 反交错：{ vp-menu-deinterlace-options.on }
    .off = 反交错：{ vp-menu-deinterlace-options.off }
