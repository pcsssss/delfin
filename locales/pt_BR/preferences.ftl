prefs-vp-on-left-click-options =
    .play-pause = Reproduzir/Pausar vídeo
    .toggle-controls = Mostrar/Ocultar controles
prefs-vp-experimental =
    .title = Preferências experimentais
    .subtitle =
        Estas preferências podem não funcionar ou estar incompletas.
        Não é recomendado modificá-las.
prefs-vp-interface = Interface
prefs-general-page = Geral
prefs-skip-amount =
    { $seconds } { $seconds ->
        [one] segundo
       *[other] segundos
    }
prefs-window-title = Preferências
prefs-vp-subs = Legendas
prefs-vp-intro-skipper-auto-skip =
    .title = Pular aberturas automaticamente
    .subtitle = Pule aberturas sem precisar apertar o botão Pular Abertura
prefs-general-language =
    .title = Idioma
    .subtitle = Contribua com as traduções em <a href="{ $weblateUrl }">Weblate</a>
    .option-default = Padrão do Sistema ({ $languageId })
prefs-vp-subs-position =
    .title = Posição da legenda
    .subtitle = Onde 0 é o canto superior da tela, e 100 é o canto inferior
prefs-vp-subs-scale =
    .title = Escala da legenda
    .subtitle = Fator de escala para o texto de legenda
prefs-vp-skip-forwards =
    .title = Quanto avançar
    .subtitle = Quantos segundos avançar de cada vez
prefs-vp-subs-style-ass-warning =
    Estas configurações não se aplicam a legendas ASS/SSA.
    Para legendas ASS/SSA, veja "{ prefs-vp-subs-more }".
prefs-vp-jellyscrub =
    .title = Plugin Jellyscrub
    .subtitle =
        Mostra miniaturas ao passar o mouse sobre a barra de progressão de vídeo.
        Isso requer que o plugin <a href="{ $jellyscrubUrl }">Jellyscrub</a> esteja instalado no seu servidor.
        O trickplay nativo do Jellyfin será usado em vez disso nos servidores com suporte a ele.
prefs-vp-on-left-click =
    .title = Ao clicar
    .subtitle = O que acontece ao clicar com o botão esquerdo dentro do reprodutor de vídeo
prefs-vp-subs-font =
    .title = Fonte da legenda
    .subtitle = Apenas fontes suportadas são listadas
prefs-vp-other = Outros
prefs-vp-skip-backwards =
    .title = Quanto retroceder
    .subtitle = Quantos segundos retroceder de cada vez
prefs-vp-subs-style = Estilo de legenda
prefs-vp-hls-playback =
    .title = Reprodução HLS
    .subtitle = Isso pode quebrar o funcionamento da seleção de trilhas de áudio e legendas.
prefs-vp-subs-colour =
    .title = Cor do texto da legenda
prefs-vp-subs-background-colour =
    .title = Cor do fundo da legenda
prefs-vp-page = Reprodutor de Vídeo
prefs-vp-backend =
    .title = Backend do reprodutor de vídeo
    .subtitle = Requer que o { app-name } seja reiniciado.
    .value-mpv = MPV
    .value-gstreamer = GStreamer
prefs-vp-subs-reset =
    .label = Redefinir
    .tooltip = Redefinir configurações de legendas
prefs-vp-plugins = Plugins
prefs-general-theme =
    .title = Tema
    .option-default = Padrão do Sistema
    .option-light = Claro
    .option-dark = Escuro
prefs-vp-subs-more-ass-warning =
    .title = Estas preferências irão afetar legendas ASS/SSA!
    .subtitle =
        Legendas ASS/SSA frequentemente incluem estilos e posicionamentos especiais. Mudar estas preferências pode fazer com que legendas ASS/SSA sejam renderizadas incorretamente..

        Se deseja apenas mudar o estilo para outros formatos de legenda, é recomendado que mude as preferências em "{ prefs-vp-subs-style }" em vez disso.
prefs-vp-subs-more = Mais preferências de legendas
prefs-vp-intro-skipper =
    .title = Plugin Intro Skipper
    .subtitle =
        Mostra um botão Pular Abertura durante as aberturas de episódios.
        Isso requer que o plugin <a href="{ $introSkipperUrl }">Intro Skipper</a> esteja instalado no seu servidor.
prefs-general-restore-most-recent =
    .title = Conectar-se automaticamente ao iniciar o { app-name }
    .subtitle = Ao iniciar, você se conectará com última conta utilizada
prefs-general-use-episode-image =
    .title = Usar imagens de episódio nas seções A Seguir e Continue Assistindo
    .subtitle = A seções A Seguir e Continue Assistindo usarão imagens de episódio como miniaturas, em vez da miniatura principal da série..
prefs-vp-mpv-hwdec =
    .title = Decodificação por hardware
    .subtitle =
        A decodificação por hardware pode melhorar a performance de reprodução.
        Escolha "{ prefs-vp-mpv-hwdec-options.auto-safe }" ou "{ prefs-vp-mpv-hwdec-options.off }" se estiver tendo problemas.
        Para mais informações, confira o <a href="{ $mpvManualUrl }">manual do MPV</a>.
prefs-vp-mpv-hwdec-options =
    .auto = Automática
    .auto-copy = Automática (cópia)
    .auto-safe = Automática (segura)
    .custom = Personalizada
    .off = Desativada
