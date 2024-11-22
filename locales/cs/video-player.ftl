vp-volume-mute-tooltip =
    { $muted ->
        [true] Zrušit ztlumení
       *[false] Ztlumit
    }
vp-unnamed-track = Nepojmenovaná stopa
vp-subtitle-track-tooltip =
    { $subtitlesAvailable ->
        [true] Stopy titulků
       *[false] Nejsou dostupné žádné titulky
    }
vp-subtitle-track-off = Vypnuto
vp-backend-gst-track-name = { $displayName } – { $title }
vp-audio-track-tooltip = Zvukové stopy
vp-no-subtitles-available = Nejsou dostupné žádné titulky.
vp-audio-track-menu = Zvuková stopa
vp-duration-tooltip =
    .total = Přepnout na zbývající čas
    .remaining = Přepnout na celkový čas
vp-next-prev-episode-tooltip =
    { $direction ->
        [next] Další
       *[previous] Předchozí
    } epizoda
vp-unnamed-item = Nepojmenovaná položka
vp-play-pause-tooltip =
    { $playing ->
        [true] Pozastavit
       *[false] Přehrát
    }
vp-fullscreen-tooltip =
    { $enter ->
        [true] Přepnout na
       *[false] Opustit
    } celou obrazovku
vp-skip-forwards-backwards-tooltip =
    Přeskočit { $direction ->
        [forwards] vpřed
       *[backwards] zpět
    } o { $seconds } { $seconds ->
        [one] sekundu
        [few] sekundy
       *[other] sekund
    }
vp-subtitle-track-external = Externí stopa titulků
vp-skip-intro =
    .manual = Přeskočit znělku
    .auto = Znělka bude přeskočena za { $seconds }…
vp-subtitle-track-menu = Stopa titulků
vp-next-up-starting =
    Další epizoda začíná za { $remaining } { $remaining ->
        [one] sekundu
        [few] sekundy
       *[other] sekund
    }…
vp-backend-mpv-track-name =
    .title-and-language = { $title } – { $language }
    .id-and-language = Stopa { $id } – { $language }
    .id = Stopa { $id }
vp-next-up-action =
    .play = Přehrát nyní
    .hide = Skrýt
vp-playback-speed-tooltip = Rychlost přehrávání
vp-playback-speed-normal = Normální
vp-playback-speed-toast = Rychlost přehrávání: { $speed }x
vp-menu-hwdec = Hardwarové dekódování
vp-menu-debug = Ladění
vp-menu-debug-copy-stream-url = Kopírovat adresu streamu
vp-menu-deinterlace = Odstranění prokládání
vp-menu-deinterlace-toast =
    .on = Odstranění prokládání: { vp-menu-deinterlace-options.on }
    .off = Odstranění prokládání: { vp-menu-deinterlace-options.off }
vp-menu-deinterlace-options =
    .on = Zapnuto
    .off = Vypnuto
    .auto = Automaticky
vp-menu-debug-copy-stream-url-toast =
    .success = Adresa streamu zkopírována do schránky
    .fail = Nepodařilo se zkopírovat adresu streamu do schránky
vp-menu-debug-mpv-props = Vlastnosti MPV
