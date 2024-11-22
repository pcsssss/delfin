prefs-vp-on-left-click-options =
    .play-pause = Přehrát/pozastavit video
    .toggle-controls = Zobrazit/skrýt ovládání
prefs-vp-experimental =
    .title = Experimentální možnosti
    .subtitle =
        Tyto možnosti mohou být rozbité nebo nedokončené.
        Jejich úprava se nedoporučuje.
prefs-vp-interface = Rozhraní
prefs-general-page = Obecné
prefs-skip-amount =
    { $seconds } { $seconds ->
        [one] sekunda
        [few] sekund
       *[other] sekund
    }
prefs-window-title = Předvolby
prefs-vp-subs = Titulky
prefs-vp-intro-skipper-auto-skip =
    .title = Automaticky přeskakovat znělky
    .subtitle = Přeskakovat znělky bez potřeby zmáčknutí tlačítka Přeskočit
prefs-general-language =
    .title = Jazyk
    .subtitle = Pomozte s překladem na <a href="{ $weblateUrl }">Weblate</a>
    .option-default = Podle systému ({ $languageId })
prefs-vp-subs-position =
    .title = Pozice titulků
    .subtitle = Kde 0 je horní a 100 spodní část obrazovky
prefs-vp-subs-scale =
    .title = Velikost titulků
    .subtitle = Faktor měřítka pro text titulků
prefs-vp-skip-forwards =
    .title = Čas k přeskočení vpřed
    .subtitle = Počet sekund k přeskočení vpřed v čase
prefs-vp-subs-style-ass-warning =
    Tato nastavení neplatí pro titulky ASS/SSA.
    Pro tento typ titulků viz „{ prefs-vp-subs-more }“.
prefs-vp-jellyscrub =
    .title = Doplněk Jellyscrub
    .subtitle =
        Zobrazit náhledy při přejetí myší přes lištu přehrávače videa.
        Tato možnost vyžaduje nainstalovaný doplněk <a href="{ $jellyscrubUrl }">Jellyscrub</a> na vašem serveru.
        Na podporovaných serverech se místo toho použije nativní trikplay Jellyfin.
prefs-vp-on-left-click =
    .title = Po kliknutí myší
    .subtitle = Co se stane po kliknutí myší uvnitř přehrávače videa
prefs-vp-subs-font =
    .title = Písmo titulků
    .subtitle = Jsou zobrazena pouze podporovaná písma
prefs-vp-other = Další
prefs-vp-skip-backwards =
    .title = Čas k přeskočení zpátky
    .subtitle = Počet sekund k přeskočení zpět v čase
prefs-vp-subs-style = Styl titulků
prefs-vp-hls-playback =
    .title = Přehrávání pomocí HLS
    .subtitle = { "." }title = Toto může rozbít výběr zvukové stopy a stopy titulků.
prefs-vp-subs-colour =
    .title = Barva textu titulků
prefs-vp-subs-background-colour =
    .title = Barva pozadí titulků
prefs-vp-page = Přehrávač
prefs-vp-backend =
    .title = Backend přehrávače videa
    .subtitle = Vyžaduje restart aplikace { app-name }
    .value-mpv = MPV
    .value-gstreamer = GStreamer
prefs-vp-subs-reset =
    .label = Resetovat
    .tooltip = Resetovat nastavení titulků
prefs-vp-plugins = Doplňky
prefs-general-theme =
    .title = Motiv
    .option-default = Podle systému
    .option-light = Světlý
    .option-dark = Tmavý
prefs-vp-subs-more-ass-warning =
    .title = Tyto možnosti ovlivní titulky ASS/SSA!
    .subtitle =
        Titulky ASS/SSA často obsahují speciální styl a umístění. Změna těchto předvoleb může způsobit nesprávné vykreslení těchto titulků.

        Pokud chcete změnit styl pouze pro jiné formáty titulků, doporučujeme místo toho změnit předvolby v části „{ prefs-vp-subs-style }“.
prefs-vp-subs-more = Další možnosti titulků
prefs-vp-intro-skipper =
    .title = Doplněk přeskočení znělky
    .subtitle =
        Zobrazí tlačítko k přeskočení během znělky epizody seriálu.
        Tato možnost vyžaduje nainstalovaný doplněk <a href="{ $introSkipperUrl }">Intro Skipper</a> na vašem serveru.
prefs-general-restore-most-recent =
    .title = Automaticky se přihlásit po spuštění aplikace { app-name }
    .subtitle = Po spuštění budete přihlášeni do vašeho naposledy použitého účtu
prefs-general-use-episode-image =
    .title = Použít obrázky epizod v sekcích Další díly a Pokračujte ve sledování
    .subtitle = Sekce Další díly a Pokračujte ve sledování budou používat obrázky epizod jako náhledy namísto primárního náhledu seriálu.
prefs-vp-mpv-hwdec-options =
    .auto = Automaticky
    .auto-copy = Automaticky (kopie)
    .auto-safe = Automaticky (bezpečný)
    .custom = Vlastní
    .off = Vypnuto
prefs-vp-mpv-hwdec =
    .title = Hardwarové dekódování
    .subtitle =
        Hardwarové dekódování můe zlepšit výkon přehrávání.
        Zvolte „{ prefs-vp-mpv-hwdec-options.auto-safe }“ nebo „{ prefs-vp-mpv-hwdec-options.off }“, pokud jste narazili problémy.
        Pro další informace navštivte <a href="{ $mpvManualUrl }">manuál MPV</a>.
