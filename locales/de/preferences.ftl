prefs-vp-on-left-click-options =
    .play-pause = Video abspielen/anhalten
    .toggle-controls = Steuerelemente anzeigen/ausblenden
prefs-vp-experimental =
    .title = Experimentelle Einstellungen
    .subtitle =
        Diese Einstellungen könnten fehlerhaft oder unvollständig sein.
        Es wird empfohlen, sie nicht zu ändern.
prefs-vp-interface = Benutzeroberfläche
prefs-general-page = Allgemein
prefs-skip-amount =
    { $seconds } { $seconds ->
        [one] Sekunde
       *[other] Sekunden
    }
prefs-window-title = Einstellungen
prefs-vp-subs = Untertitel
prefs-vp-intro-skipper-auto-skip =
    .title = Vorspann automatisch überspringen
    .subtitle = Überspringe den Vorspann, ohne dass die entsprechende Taste gedrückt werden muss
prefs-general-language =
    .title = Sprache
    .subtitle = Übersetzungen auf <a href="{ $weblateUrl }">Weblate</a> beisteuern
    .option-default = Systemstandard ({ $languageId })
prefs-vp-subs-position =
    .title = Untertitelposition
    .subtitle = 0 entspricht dem oberen Bildschirmrand und 100 dem unteren
prefs-vp-subs-scale =
    .title = Untertitelgröße
    .subtitle = Skalierungsfaktor für Untertiteltext
prefs-vp-skip-forwards =
    .title = Sprungweite vorwärts
    .subtitle = Wie viele Sekunden wird auf einmal vorgespult
prefs-vp-subs-style-ass-warning =
    Diese Einstellungen beziehen sich nicht auf ASS/SSA-Untertitel.
    Für ASS/SSA-Untertitel, siehe "{ prefs-vp-subs-more }".
prefs-vp-jellyscrub =
    .title = Jellyscrub-Plugin
    .subtitle =
        Zeige Vorschaubilder wenn du mit dem Mauszeiger über die Wiedergabeleiste fährst.
        Hierfür muss das <a href="{ $jellyscrubUrl }">Jellyscrub</a>-Plugin auf deinem Server installiert sein.
        Ansonsten wird Jellyfin’s natives Trickplay auf unterstützten Servern benutzt werden.
prefs-vp-on-left-click =
    .title = Aktion bei Mausklick
    .subtitle = Was passiert bei einem Linksklick im Videoplayer
prefs-vp-subs-font =
    .title = Schriftart
    .subtitle = Es werden nur unterstützte Schriftarten angezeigt
prefs-vp-other = Andere
prefs-vp-skip-backwards =
    .title = Sprungweite rückwärts
    .subtitle = Wie viele Sekunden wird auf einmal zurückgespult
prefs-vp-subs-style = Untertiteldesign
prefs-vp-hls-playback =
    .title = HLS-Wiedergabe
    .subtitle = Die Auswahl von Tonspuren und Untertiteln könnte dadurch beeinträchtigt werden.
prefs-vp-subs-colour =
    .title = Untertitel-Textfarbe
prefs-vp-subs-background-colour =
    .title = Untertitel-Hintergrundfarbe
prefs-vp-page = Videoplayer
prefs-vp-backend =
    .title = Interner Videoplayer
    .subtitle = Erfordert einen Neustart von { app-name }.
    .value-mpv = MPV
    .value-gstreamer = GStreamer
prefs-vp-subs-reset =
    .label = Zurücksetzen
    .tooltip = Untertiteleinstellungen zurücksetzen
prefs-vp-plugins = Plugins
prefs-general-theme =
    .title = Theme
    .option-default = Systemstandard
    .option-light = Hell
    .option-dark = Dunkel
prefs-vp-subs-more-ass-warning =
    .title = Diese Einstellungen beziehen sich auf ASS/SSA-Untertitel!
    .subtitle =
        ASS/SSA-Untertitel verwenden oft ein spezielle Design. Das Ändern dieser Einstellungen kann dazu führen, dass ASS/SSA-Untertitel inkorrekt angezeigt werden.

        Wenn du das Aussehen für andere Untertitelformate ändern willst, solltest du die Option "{ prefs-vp-subs-style }" verwenden.
prefs-vp-subs-more = Mehr Untertiteleinstellungen
prefs-vp-intro-skipper =
    .title = Intro Skipper-Plugin
    .subtitle =
        Zeige während dem Vorspann einer Folge eine Taste an, um den Vorspann zu überspringen.
        Hierfür muss das <a href="{ $introSkipperUrl }">Intro Skipper</a>-Plugin auf deinem Server installiert sein.
prefs-general-restore-most-recent =
    .title = Beim Start von { app-name } automatisch anmelden
    .subtitle = Beim Start wirst du mit dem letzten verwendeten Konto angemeldet
prefs-general-use-episode-image =
    .title = Episodenbilder in den Abschnitten „Nächste“ und „Weitergucken“ verwenden
    .subtitle = „Nächste“ und „Weitergucken“ werden Epoisodenbilder als Thumbnails benutzen, und nicht das Haupt-Thumbnail der Serie.
