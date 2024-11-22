prefs-general-page = Général
prefs-window-title = Préférences
prefs-general-language =
    .title = Langues
    .subtitle = Contribuez aux traductions sur <a href="{ $weblateUrl }">Weblate</a>
    .option-default = Réglage du système par défaut ({ $languageId })
prefs-vp-page = Lecteur vidéo
prefs-general-theme =
    .title = Thème
    .option-default = Réglages systèmes par défaut
    .option-light = Clair
    .option-dark = Sombre
prefs-vp-plugins = Plugins
prefs-vp-subs-style = Style des sous-titres
prefs-vp-other = Autre
prefs-vp-subs-more = Plus de préférences pour les sous-titres
prefs-vp-subs = Sous-titres
prefs-general-restore-most-recent =
    .title = Se connecter automatiquement quand { app-name } se lance
    .subtitle = Au lancement, vous serez connecté à votre dernier compte utilisé
prefs-vp-interface = Interface
prefs-vp-on-left-click =
    .title = Quand la souris est cliquée
    .subtitle = Ce qu'il ce passe quand vous cliquez gauche dans le lecteur vidéo
prefs-vp-on-left-click-options =
    .play-pause = Lise/Pause vidéo
    .toggle-controls = Afficher/ Cacher les controles
prefs-vp-subs-reset =
    .label = Réinitialiser
    .tooltip = Réinitialiser les paramètres des sous-titres
prefs-vp-subs-style-ass-warning =
    Ces paramètres ne s'appliquent pas aux sous-titres ASS/SSA.
    Pour les sous-titres ASS/SSA, voir "{ prefs-vp-subs-more }".
prefs-vp-subs-colour =
    .title = Couleur du texte des sous-titres
prefs-vp-subs-background-colour =
    .title = Couleur d'arrière-plan des sous-titres
prefs-vp-subs-font =
    .title = Police des sous-titres
    .subtitle = Seules les polices prises en charge sont répertoriées
prefs-skip-amount =
    { $seconds } { $seconds ->
        [one] seconde
       *[other] secondes
    }
prefs-vp-skip-forwards =
    .title = Le temps de l'avance rapide
    .subtitle = Le nombre de secondes de l'avance rapide
prefs-vp-skip-backwards =
    .title = Le temps de rembobinage
    .subtitle = Le nombre de secondes du rembobinage
prefs-vp-backend =
    .title = Lecteur vidéo en arrière-plan
    .subtitle = { app-name } doit être relancé.
    .value-mpv = MPV
    .value-gstreamer = GStreamer
prefs-vp-jellyscrub =
    .title = Jellyscrub plugin
    .subtitle =
        Affiche les vignettes en survolant la barre de progression de la vidéo
        Il faut que le plugin <a href="{ $jellyscrubUrl }">Jellyscrub</a> soit installé sur votre serveur.
        La fonction trickplay native de Jellyfin sera utilisée à la place sur les serveurs pris en charge.
prefs-general-use-episode-image =
    .title = Utiliser les images d'épisode dans les sections À Suivre et Continuer à regarder
    .subtitle = Les sections À Suivre et Continuer à regarder utiliseront des images d'épisode comme vignettes au lieu de la principale de l'émission.
prefs-vp-subs-scale =
    .title = Échelle des sous-titres
    .subtitle = Facteur d'échelle pour le texte des sous-titres
prefs-vp-subs-position =
    .title = Position du sous-titre
    .subtitle = Où 0 est le haut de l'écran et 100 est le bas
prefs-vp-intro-skipper =
    .title = Intro Skipper plugin
    .subtitle =
        Affiche un bouton « Skip Intro » pendant les introductions d'épisodes.
        Il faut que le plugin <a href="{ $introSkipperUrl }">Intro Skipper</a> soit installé sur votre serveur.
prefs-vp-intro-skipper-auto-skip =
    .title = Saut automatique des intros
    .subtitle = Saut des intros sans avoir besoins d'appuyer sur le bouton de saut
prefs-vp-experimental =
    .title = Les préférences expérimentales
    .subtitle =
        Ces préférences peuvent être cassées ou incomplètes.
        Il n'est pas recommandé de les modifier.
prefs-vp-hls-playback =
    .title = Lecture HLS
    .subtitle = Cela peut casser la sélection des pistes audio et des sous-titres.
prefs-vp-subs-more-ass-warning =
    .title = Ces préférences affecteront les sous-titres ASS/SSA !
    .subtitle =
        Les sous-titres ASS/SSA comprennent souvent un style et un positionnement spéciaux. La modification de ces préférences peut entraîner un rendu incorrect des sous-titres ASS/SSA.

        Si vous souhaitez uniquement modifier le style pour d'autres formats de sous-titres, il est recommandé de modifier les préférences sous "{ prefs-vp-subs-style }" à la place.
prefs-vp-mpv-hwdec-options =
    .auto = Automatique
    .auto-copy = Automatique (copie)
    .auto-safe = Automatique (sûr)
    .custom = Custom
    .off = Désactivé
prefs-vp-mpv-hwdec =
    .title = Décodage matériel
    .subtitle =
        Le décodage matériel peut améliorer les performances de lecture.
        Choisissez "{ prefs-vp-mpv-hwdec-options.auto-safe }" ou "{ prefs-vp-mpv-hwdec-options.off }" si vous rencontrez des problèmes.
        Pour plus d'informations, référez-vous au <a href="{ $mpvManualUrl }">manuel de MPV</a>.
