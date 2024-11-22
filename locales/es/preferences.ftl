prefs-vp-on-left-click-options =
    .play-pause = Reproducir/pausar video
    .toggle-controls = Mostrar/ocultar controles
prefs-vp-experimental =
    .title = Preferencias experimentales
    .subtitle =
        Estas preferencias pueden estar rotas o incompletas.
        No se recomienda modificarlas.
prefs-vp-interface = Interfaz
prefs-general-page = General
prefs-skip-amount =
    { $seconds } { $seconds ->
        [one] segundo
       *[other] segundos
    }
prefs-window-title = Preferencias
prefs-vp-subs = Subtítulos
prefs-vp-intro-skipper-auto-skip =
    .title = Saltar intros automáticamente
    .subtitle = Saltar intros sin tener que pulsar el botón «Saltar Intro»
prefs-general-language =
    .title = Idioma
    .subtitle = Contribuye a las traducciones en <a href="{ $weblateUrl }">Weblate</a>
    .option-default = Predeterminado del sistema ({ $languageId })
prefs-vp-subs-position =
    .title = Posición de los subtítulos
    .subtitle = Donde 0 es la parte superior de la pantalla, y 100 es la parte inferior
prefs-vp-subs-scale =
    .title = Escala de los subtítulos
    .subtitle = Factor de escala para el texto de los subtítulos
prefs-vp-skip-forwards =
    .title = Porcentaje de salto hacia adelante
    .subtitle = Cuántos segundos avanzar cada vez
prefs-vp-subs-style-ass-warning =
    Estos ajustes no se aplican a los subtítulos ASS/SSA.
    Para los subtítulos ASS/SSA, consulta "{ prefs-vp-subs-more }".
prefs-vp-jellyscrub =
    .title = complemento Jellyscrub
    .subtitle =
        Mostrar miniaturas al pasar el cursor sobre la barra de progreso del video.
        Esto requiere que el complemento <a href="{ $jellyscrubUrl }">Jellyscrub</a> esté instalado en su servidor.
        En los servidores compatibles se utilizará el truco nativo de Jellyfin.
prefs-vp-on-left-click =
    .title = Al hacer clic con el ratón
    .subtitle = Qué ocurre cuando haces clic con el botón izquierdo del ratón en el reproductor de vídeo
prefs-vp-subs-font =
    .title = Fuente del subtítulo
    .subtitle = Sólo se muestran las fuentes admitidas
prefs-vp-other = Otros
prefs-vp-skip-backwards =
    .title = Porcentaje de salto hacia atrás
    .subtitle = Cuántos segundos retroceder cada vez
prefs-vp-subs-style = Estilo de subtítulos
prefs-vp-hls-playback =
    .title = Reproducción HLS
    .subtitle = Puede interrumpir la selección de pistas de audio y subtítulos.
prefs-vp-subs-colour =
    .title = Color del texto del subtítulo
prefs-vp-subs-background-colour =
    .title = Color de fondo del subtítulo
prefs-vp-page = Reproductor de video
prefs-vp-backend =
    .title = Backend del reproductor de vídeo
    .subtitle = Require reiniciar { app-name }.
    .value-mpv = MPV
    .value-gstreamer = GStreamer
prefs-vp-subs-reset =
    .label = Restablecer
    .tooltip = Restablecer ajustes de subtítulos
prefs-vp-plugins = Plugins
prefs-general-theme =
    .title = Tema
    .option-default = Predeterminado del sistema
    .option-light = Claro
    .option-dark = Oscuro
prefs-vp-subs-more-ass-warning =
    .title = Estas preferencias afectarán a los subtítulos ASS/SSA.
    .subtitle =
        Los subtítulos ASS/SSA suelen incluir estilos y posiciones especiales. Cambiar estas preferencias puede hacer que los subtítulos ASS/SSA no se muestren correctamente.

        Si sólo deseas cambiar el estilo para otros formatos de subtítulos, se recomienda que cambies las preferencias en "{ prefs-vp-subs-style }" en su lugar.
prefs-vp-subs-more = Más preferencias de subtítulos
prefs-vp-intro-skipper =
    .title = Plugin Intro Skipper
    .subtitle =
        Muestra un botón de «Saltar Intro» durante las intros de los episodios.
        Esto requiere que el plugin <a href="{ $introSkipperUrl }">Intro Skipper</a> esté instalado en tu servidor.
prefs-general-restore-most-recent =
    .title = Iniciar sesión automáticamente al iniciarse { app-name }
    .subtitle = Al iniciarse, iniciará sesión con la cuenta que haya utilizado más recientemente
prefs-general-use-episode-image =
    .title = Utilizar las imágenes de los episodios en las secciones "Próximamente" y "Seguir viendo
    .subtitle = Las secciones Próximamente y Seguir viendo utilizarán imágenes de episodios como miniaturas en lugar de la miniatura principal del programa.
prefs-vp-mpv-hwdec-options =
    .auto = Automático
    .auto-copy = Automático (copia)
    .auto-safe = Automático (segura)
    .custom = Personalizada
    .off = Apagada
prefs-vp-mpv-hwdec =
    .title = Decodificación de hardware
    .subtitle =
        La decodificación por hardware puede mejorar el rendimiento de reproducción.
        Elija "{ prefs-vp-mpv-hwdec-options.auto-safe }" o "{ prefs-vp-mpv-hwdec-options.off }" si tiene problemas.
        Para obtener más información, consulte el <a href="{ $mpvManualUrl }">manual de MPV</a>.
