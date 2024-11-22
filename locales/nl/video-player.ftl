vp-volume-mute-tooltip =
    { $muted ->
        [true] Ontdempen
       *[false] Dempen
    }
vp-unnamed-track = Naamloos spoor
vp-subtitle-track-tooltip =
    { $subtitlesAvailable ->
        [true] Ondertitelsporen
       *[false] Er zijn geen ondertitelsporen beschikbaar
    }
vp-subtitle-track-off = Uit
vp-backend-gst-track-name = { $displayName } – { $title }
vp-audio-track-tooltip = Audiosporen
vp-no-subtitles-available = Er is geen ondertiteling beschikbaar.
vp-audio-track-menu = Audiospoor
vp-duration-tooltip =
    .total = Resterende tijd tonen
    .remaining = Eindtijd tonen
vp-next-prev-episode-tooltip =
    { $direction ->
        [next] Volgende
       *[previous] Vorige
    } episode
vp-unnamed-item = Naamloos item
vp-play-pause-tooltip =
    { $playing ->
        [true] Pauzeren
       *[false] Afspelen
    }
vp-fullscreen-tooltip =
    { $enter ->
        [true] Aan
       *[false] Uit
    } fullscreen
vp-skip-forwards-backwards-tooltip =
    Skip { $direction ->
        [forwards] vooruitspoelen
       *[backwards] terugspoelen
    } { $seconds } { $seconds ->
        [one] seconde
       *[other] seconden
    }
vp-subtitle-track-external = Extern ondertitelspoor
vp-skip-intro =
    .manual = Introductie overslaan
    .auto = De introductie wordt over { $seconds } overgeslagen…
vp-subtitle-track-menu = Ondertitelspoor
vp-next-up-starting =
    De volgende aflevering begint over { $remaining } { $remaining ->
        [one] seconde
       *[other] seconden
    }…
vp-backend-mpv-track-name =
    .title-and-language = { $title } – { $language }
    .id-and-language = Spoor { $id } – { $language }
    .id = Spoor { $id }
vp-next-up-action =
    .play = Nu afspelen
    .hide = Verbergen
vp-playback-speed-tooltip = Afspeelsnelheid
vp-playback-speed-normal = Normaal
vp-playback-speed-toast = Afspeelsnelheid: { $speed }x
vp-menu-hwdec = Hardwareversnelling
vp-menu-deinterlace = Deinterlacing
vp-menu-deinterlace-toast =
    .on = Deinterlacing: { vp-menu-deinterlace-options.on }
    .off = Deinterlacing: { vp-menu-deinterlace-options.off }
vp-menu-deinterlace-options =
    .on = Aan
    .off = Uit
    .auto = Automatisch
vp-menu-debug = Foutopsporing
vp-menu-debug-copy-stream-url = Stream-url kopiëren
vp-menu-debug-copy-stream-url-toast =
    .success = De stream-url is gekopieerd naar het klembord
    .fail = De stream-url kan niet worden gekopieerd
vp-menu-debug-mpv-props = Mpv-eigenschappen
