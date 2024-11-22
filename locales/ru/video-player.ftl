vp-volume-mute-tooltip =
    { $muted ->
        [true] Включить звук
       *[false] Выключить звук
    }
vp-unnamed-track = Дорожка без названия
vp-subtitle-track-tooltip =
    { $subtitlesAvailable ->
        [true] Субтитры
       *[false] Субтитры недоступны
    }
vp-subtitle-track-off = Выключить
vp-backend-gst-track-name = { $displayName } – { $title }
vp-audio-track-tooltip = Звуковые дорожки
vp-no-subtitles-available = Субтитры недоступны.
vp-audio-track-menu = Звуковая дорожка
vp-duration-tooltip =
    .total = Переключить на оставшееся время
    .remaining = Переключить на общую длительность
vp-next-prev-episode-tooltip =
    { $direction ->
        [next] Следующая
       *[previous] Предыдущая
    } серия
vp-unnamed-item = Элемент без названия
vp-play-pause-tooltip =
    { $playing ->
        [true] Пауза
       *[false] Воспроизведение
    }
vp-fullscreen-tooltip =
    { $enter ->
        [true] Развернуть на весь экран
       *[false] Выйти из полного экрана
    }
vp-skip-forwards-backwards-tooltip =
    Перемотать { $direction ->
        [forwards] вперёд
       *[backwards] назад
    } { $seconds } { $seconds ->
        [one] сек.
       *[other] сек.
    }
vp-subtitle-track-external = Внешние субтитры
vp-skip-intro =
    .manual = Пропустить заставку
    .auto = Пропуск заставки через { $seconds }…
vp-subtitle-track-menu = Субтитры
vp-next-up-starting =
    Следующая серия начнётся через { $remaining } { $remaining ->
        [one] сек.
       *[other] сек.
    }…
vp-backend-mpv-track-name =
    .title-and-language = { $title } – { $language }
    .id-and-language = Дорожка { $id } – { $language }
    .id = Дорожка { $id }
vp-next-up-action =
    .play = Смотреть сейчас
    .hide = Скрыть
vp-playback-speed-tooltip = Скорость воспроизведения
vp-playback-speed-normal = Нормальная
vp-menu-hwdec = Аппаратное декодирование
vp-menu-deinterlace-options =
    .on = Включено
    .off = Выключено
    .auto = Авто
vp-playback-speed-toast = Скорость воспроизведения: { $speed }x
