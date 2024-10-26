prefs-window-title = Preferences
prefs-general-page = General
prefs-vp-page = Video Player

# General
# =======

prefs-general-language =
    .title = Language
    .subtitle = Contribute to translations on <a href="{ $weblateUrl }">Weblate</a>
    .option-default = System Default ({ $languageId })

prefs-general-theme =
    .title = Theme
    .option-default = System Default
    .option-light = Light
    .option-dark = Dark

prefs-general-restore-most-recent =
    .title = Sign in automatically when { app-name } starts
    .subtitle = On startup, you will be signed in to your most recently used account

prefs-general-use-episode-image =
    .title = Use episode images in Next Up and Continue Watching sections
    .subtitle = Next Up and Continue Watching sections will use episode images as thumbnails instead of the primary thumbnail of the show.

# Interface
# =========

prefs-vp-interface = Interface
prefs-skip-amount =
    { $seconds } { $seconds ->
        [one] second
       *[other] seconds
    }
prefs-vp-skip-backwards =
    .title = Skip backwards amount
    .subtitle = How many seconds to skip backwards at a time
prefs-vp-skip-forwards =
    .title = Skip forwards amount
    .subtitle = How many seconds to skip forwards at a time
prefs-vp-on-left-click =
    .title = When the mouse is clicked
    .subtitle = What happens when you left click inside the video player
prefs-vp-on-left-click-options =
    .play-pause = Play/pause video
    .toggle-controls = Show/hide controls

# Subtitles
# =========

prefs-vp-subs = Subtitles
prefs-vp-subs-reset =
    .label = Reset
    .tooltip = Reset subtitle settings

prefs-vp-subs-style = Subtitle style
prefs-vp-subs-style-ass-warning =
    These settings don't apply to ASS/SSA subtitles.
    For ASS/SSA subtitles, see "{prefs-vp-subs-more}".

prefs-vp-subs-colour =
    .title = Subtitle text color
prefs-vp-subs-background-colour =
    .title = Subtitle background color
prefs-vp-subs-font =
    .title = Subtitle font
    .subtitle = Only supported fonts are listed

prefs-vp-subs-more = More subtitle preferences
prefs-vp-subs-more-ass-warning =
    .title = These preferences will affect ASS/SSA subtitles!
    .subtitle =
        ASS/SSA subtitles often include special styling and positioning. Changing these preferences may cause ASS/SSA subtitles to render incorrectly.

        If you only want to change the style for other subtitle formats, it's recommended that you change the preferences under "{prefs-vp-subs-style}" instead.

prefs-vp-subs-scale =
    .title = Subtitle scale
    .subtitle = Scaling factor for subtitle text
prefs-vp-subs-position =
    .title = Subtitle position
    .subtitle = Where 0 is the top of the screen, and 100 is the bottom

# Plugins
# =======

prefs-vp-plugins = Plugins
prefs-vp-intro-skipper =
    .title = Intro Skipper plugin
    .subtitle =
        Shows a Skip Intro button during episode intros.
        This requires the <a href="{ $introSkipperUrl }">Intro Skipper</a> plugin to be installed on your server.
prefs-vp-intro-skipper-auto-skip =
    .title = Automatically skip intros
    .subtitle = Skip intros without having to press the Skip Intro button
prefs-vp-jellyscrub =
    .title = Jellyscrub plugin
    .subtitle =
        Show thumbnails while hovering over the video progress bar.
        This requires the <a href="{ $jellyscrubUrl }">Jellyscrub</a> plugin to be installed on your server.
        Jellyfin's native trickplay will be used instead on supported servers.

# Other
# =====

prefs-vp-other = Other
prefs-vp-mpv-hwdec =
    .title = Hardware decoding
    .subtitle =
        Hardware decoding can improve playback performance.
        Choose "{ prefs-vp-mpv-hwdec-options.auto-safe }" or "{ prefs-vp-mpv-hwdec-options.off }" if you're experiencing issues.
        For more info, refer to the <a href="{ $mpvManualUrl }">MPV manual</a>.
prefs-vp-mpv-hwdec-options =
    .auto = Auto
    .auto-copy = Auto (copy)
    .auto-safe = Auto (safe)
    .custom = Custom
    .off = Off
prefs-vp-experimental =
    .title = Experimental preferences
    .subtitle =
        These preferences may be broken or incomplete.
        Modifying them is not recommended.
prefs-vp-backend =
    .title = Video player backend
    .subtitle = Requires restarting { app-name }.
    .value-mpv = MPV
    .value-gstreamer = GStreamer
prefs-vp-hls-playback =
    .title = HLS playback
    .subtitle = This may break audio and subtitle track selection.
