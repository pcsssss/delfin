vp-next-prev-episode-tooltip =
    { $direction ->
        [next] Suivant
       *[previous] Précédent
    } episode
vp-unnamed-item = Nom inconnu
vp-fullscreen-tooltip =
    { $enter ->
        [true] Entrer
       *[false] Sortir
    } fullscreen
vp-next-up-starting =
    Prochain épisode dans { $remaining } { $remaining ->
        [one] secondes
       *[other] secondes
    }…
vp-next-up-action =
    .play = Jouer
    .hide = Cacher
vp-audio-track-tooltip = Pistes Audio
vp-audio-track-menu = Piste Audio
vp-subtitle-track-tooltip =
    { $subtitlesAvailable ->
        [true] Pistes de sous-titres
       *[false] Pas de pistes de sous-titres disponibles
    }
vp-subtitle-track-menu = Piste de sous-titre
vp-subtitle-track-external = Piste de sous-titre externe
vp-subtitle-track-off = Arrêt
vp-volume-mute-tooltip =
    { $muted ->
        [true] Rétablir le son
       *[false] Couper le son
    }
vp-no-subtitles-available = Pas de sous-titres disponibles.
vp-playback-speed-tooltip = Vitesse de lecture
vp-playback-speed-normal = Normal
vp-unnamed-track = Piste sans nom
vp-play-pause-tooltip =
    { $playing ->
        [true] Pause
       *[false] Lire
    }
vp-duration-tooltip =
    .total = Remplacer par le temps restant
    .remaining = Remplacer par le temps totals
vp-skip-forwards-backwards-tooltip =
    Passer { $seconds } { $seconds ->
        [one] seconde
       *[other] secondes
    } en { $direction ->
        [forwards] avance rapide
       *[backwards] rembobinage
    }
vp-skip-intro =
    .manual = Passer l'intro
    .auto = Passe l'intro dans { $seconds }…
vp-playback-speed-toast = Vitesse de lecture : { $speed }x
vp-backend-gst-track-name = { $displayName } – { $title }
vp-backend-mpv-track-name =
    .title-and-language = { $title } – { $language }
    .id-and-language = Piste { $id } – { $language }
    .id = Piste { $id }
vp-menu-hwdec = Décodage matériel
vp-menu-debug = Débogage
vp-menu-debug-copy-stream-url = Copier l'URL du flux
vp-menu-debug-copy-stream-url-toast =
    .success = URL du flux copié au presse-papier
    .fail = Échec de l'URL du flux au presse-papier
vp-menu-deinterlace = Désentrelacement
vp-menu-deinterlace-toast =
    .on = Désentrelacement : { vp-menu-deinterlace-options.on }
    .off = Désentrelacement : { vp-menu-deinterlace-options.off }
vp-menu-deinterlace-options =
    .on = On
    .off = Off
    .auto = Auto
vp-menu-debug-mpv-props = Propriétés de MPV
