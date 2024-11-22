prefs-vp-interface = Vormgeving
prefs-general-page = Algemeen
prefs-skip-amount =
    { $seconds } { $seconds ->
        [one] seconde
       *[other] seconden
    }
prefs-window-title = Voorkeuren
prefs-general-language =
    .title = Taal
    .subtitle = Help met vertalen op <a href="{ $weblateUrl }">Weblate</a>
    .option-default = Systeemtaal ({ $languageId })
prefs-vp-page = Videospeler
prefs-general-theme =
    .title = Thema
    .option-default = Systeemthema
    .option-light = Licht
    .option-dark = Donker
prefs-vp-on-left-click-options =
    .play-pause = Video afspelen/pauzeren
    .toggle-controls = Bediening tonen/verbergen
prefs-vp-experimental =
    .title = Experimentele voorkeuren
    .subtitle =
        Let op: deze voorkeuren kunnen niet-werkend of onvolledig zijn.
        Pas ze bij voorkeur niet aan.
prefs-vp-subs = Ondertiteling
prefs-vp-intro-skipper-auto-skip =
    .title = Introducties automatisch overslaan
    .subtitle = Sla introducties over zonder op de knop te drukken
prefs-vp-subs-position =
    .title = Ondertitelpositie
    .subtitle = 0 is de bovenkant van het scherm; 100 de onderkant
prefs-vp-subs-scale =
    .title = Ondertitelgrootte
    .subtitle = De vergrotingsfactor van ondertiteling
prefs-vp-skip-forwards =
    .title = Doorspoelhoeveelheid
    .subtitle = Hoeveel seconden per keer dient te worden doorgespoeld
prefs-vp-subs-style-ass-warning =
    Let op: deze voorkeuren zijn niet van toepassing op ASS-/SSA-ondertiteling.
    Zie ‘{ prefs-vp-subs-more }’.
prefs-vp-jellyscrub =
    .title = Jellyscrub
    .subtitle =
        Toon miniaturen door de cursor op de voortgangsbalk te houden.
        Hiervoor dient de <a href="{ $jellyscrubUrl }">Jellyscrub</a>-plug-in geïnstalleerd te zijn op uw server.
        Jellyfin's trickplay wordt op ondersteunde servers gebruikt.
prefs-vp-on-left-click =
    .title = Na aanklikken
    .subtitle = Welke actie dient te worden uitgevoerd na linksklikken in de videospeler
prefs-vp-subs-font =
    .title = Lettertype
    .subtitle = Er worden alleen ondersteunde lettertypen getoond
prefs-vp-other = Overig
prefs-vp-skip-backwards =
    .title = Terugspoelhoeveelheid
    .subtitle = Hoeveel seconden per keer dient te worden teruggespoeld
prefs-vp-subs-style = Ondertitelstijl
prefs-vp-hls-playback =
    .title = Afspelen met HLS
    .subtitle = Let op: mogelijk werkt de audio- en ondertitelspoorkeuze hierdoor niet.
prefs-vp-subs-colour =
    .title = Tekstkleur
prefs-vp-subs-background-colour =
    .title = Achtergrondkleur
prefs-vp-backend =
    .title = Videospelerbackend
    .subtitle = Herstart { app-name } om de wijziging toe te passen.
    .value-mpv = MPV
    .value-gstreamer = GStreamer
prefs-vp-subs-reset =
    .label = Standaardwaarden
    .tooltip = Herstel de standaardvoorkeuren
prefs-vp-plugins = Uitbreidingen
prefs-vp-subs-more-ass-warning =
    .title = Deze voorkeuren zijn van toepassing op ASS-/SSA-ondertiteling!
    .subtitle =
        Let op: ASS-/SSA-ondertiteling bevat doorgaans speciale opmaak en positionering. Door deze voorkeuren aan te passen, kan het voorkomen dat de ondertiteling niet goed getoond wordt.

        Als u alleen de stijl van andere ondertitelformaten wilt aanpassen, ga dan naar ‘{ prefs-vp-subs-style }’.
prefs-vp-subs-more = Meer ondertitelvoorkeuren
prefs-vp-intro-skipper =
    .title = Introductie overslaan
    .subtitle =
        Toon een knop om de introductie van afleveringen over te slaan.
        Hiervoor dient de <a href="{ $introSkipperUrl }">Intro Skipper</a>-plug-in geïnstalleerd te zijn op uw server.
prefs-general-restore-most-recent =
    .title = Automatisch inloggen na starten van { app-name }
    .subtitle = Log na het opstarten automatisch in op uw laatstgebruikte account
prefs-general-use-episode-image =
    .title = Afbeeldingsminiaturen gebruiken in wachtrij en verder kijken
    .subtitle = Gebruik miniaturen van de afleveringen in kwestie in de wachtrij en verder kijken in plaats van de miniatuur van de serie.
prefs-vp-mpv-hwdec-options =
    .auto = Automatisch
    .auto-copy = Automatisch (kopie)
    .auto-safe = Automatisch (veilig)
    .custom = Aangepast
    .off = Uit
prefs-vp-mpv-hwdec =
    .title = Hardwareversnelling
    .subtitle =
        Hardwareversnelling kan de afspeelprestaties verbeteren.
        Kies ‘{ prefs-vp-mpv-hwdec-options.auto-safe }’ of ‘{ prefs-vp-mpv-hwdec-options.off }’ als je problemen ervaart.
        Bekijk voor meer informatie de <a href="{ $mpvManualUrl }">MPV-handleiding</a>.
