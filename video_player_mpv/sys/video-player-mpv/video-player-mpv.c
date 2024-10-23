#include "video-player-mpv.h"
#include "video-player-mpv/track.h"

#include <epoxy/egl.h>
#include <epoxy/glx.h>
#include <gtk/gtk.h>
#include <locale.h>
#include <mpv/client.h>
#include <mpv/render.h>
#include <mpv/render_gl.h>
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#ifdef GDK_WINDOWING_WAYLAND
#include <gdk/wayland/gdkwayland.h>
#endif
#ifdef GDK_WINDOWING_X11
#include <gdk/x11/gdkx.h>
#endif

#include "video-player-mpv/track-list.h"

enum {
  SIGNAL_INITIALIZED,
  SIGNAL_POSITION_UPDATED,
  SIGNAL_DURATION_UPDATED,
  SIGNAL_TRACKS_UPDATED,
  SIGNAL_VOLUME_UPDATED,
  SIGNAL_MUTE_UPDATED,
  SIGNAL_END_OF_FILE,
  SIGNAL_SEEKING,
  SIGNAL_CORE_IDLE,
  SIGNAL_CURRENT_AO,
  SIGNAL_PAUSE,
  SIGNAL_LAST,
};

static guint signals[SIGNAL_LAST] = {
    0,
};

struct MpvCtx {
  mpv_handle *handle;
  mpv_render_context *render_context;
  int width;
  int height;
};

struct _VpmVideoPlayerMpv {
  GtkGLArea parent;
  struct MpvCtx *mpv_ctx;
};

G_DEFINE_TYPE(VpmVideoPlayerMpv, vpm_video_player_mpv, GTK_TYPE_GL_AREA);

static void vpm_video_player_mpv_class_init(VpmVideoPlayerMpvClass *klass) {
  gtk_widget_class_set_css_name(GTK_WIDGET_CLASS(klass), "VideoPlayerMpv");

  GtkCssProvider *provider = gtk_css_provider_new();
  gtk_css_provider_load_from_string(provider, "VideoPlayerMpv {"
                                              "   background: black;"
                                              "}");
  gtk_style_context_add_provider_for_display(
      gdk_display_get_default(), GTK_STYLE_PROVIDER(provider),
      GTK_STYLE_PROVIDER_PRIORITY_APPLICATION);

  signals[SIGNAL_INITIALIZED] =
      g_signal_new("initialized", G_TYPE_FROM_CLASS(klass), G_SIGNAL_RUN_LAST,
                   0, NULL, NULL, NULL, G_TYPE_NONE, 0);
  signals[SIGNAL_DURATION_UPDATED] = g_signal_new(
      "duration-updated", G_TYPE_FROM_CLASS(klass), G_SIGNAL_RUN_LAST, 0, NULL,
      NULL, NULL, G_TYPE_NONE, 1, G_TYPE_DOUBLE);
  signals[SIGNAL_POSITION_UPDATED] = g_signal_new(
      "position-updated", G_TYPE_FROM_CLASS(klass), G_SIGNAL_RUN_LAST, 0, NULL,
      NULL, NULL, G_TYPE_NONE, 1, G_TYPE_DOUBLE);
  signals[SIGNAL_TRACKS_UPDATED] = g_signal_new(
      "tracks-updated", G_TYPE_FROM_CLASS(klass), G_SIGNAL_RUN_LAST, 0, NULL,
      NULL, NULL, G_TYPE_NONE, 1, VPM_TYPE_TRACK_LIST);
  signals[SIGNAL_VOLUME_UPDATED] = g_signal_new(
      "volume-updated", G_TYPE_FROM_CLASS(klass), G_SIGNAL_RUN_LAST, 0, NULL,
      NULL, NULL, G_TYPE_NONE, 1, G_TYPE_DOUBLE);
  signals[SIGNAL_MUTE_UPDATED] =
      g_signal_new("mute-updated", G_TYPE_FROM_CLASS(klass), G_SIGNAL_RUN_LAST,
                   0, NULL, NULL, NULL, G_TYPE_NONE, 1, G_TYPE_BOOLEAN);
  signals[SIGNAL_END_OF_FILE] =
      g_signal_new("end-of-file", G_TYPE_FROM_CLASS(klass), G_SIGNAL_RUN_LAST,
                   0, NULL, NULL, NULL, G_TYPE_NONE, 0);
  signals[SIGNAL_SEEKING] =
      g_signal_new("seeking", G_TYPE_FROM_CLASS(klass), G_SIGNAL_RUN_LAST, 0,
                   NULL, NULL, NULL, G_TYPE_NONE, 1, G_TYPE_BOOLEAN);
  signals[SIGNAL_CORE_IDLE] =
      g_signal_new("core-idle", G_TYPE_FROM_CLASS(klass), G_SIGNAL_RUN_LAST, 0,
                   NULL, NULL, NULL, G_TYPE_NONE, 1, G_TYPE_BOOLEAN);
  signals[SIGNAL_CURRENT_AO] =
      g_signal_new("current-ao", G_TYPE_FROM_CLASS(klass), G_SIGNAL_RUN_LAST, 0,
                   NULL, NULL, NULL, G_TYPE_NONE, 1, G_TYPE_STRING);
  signals[SIGNAL_PAUSE] =
      g_signal_new("pause", G_TYPE_FROM_CLASS(klass), G_SIGNAL_RUN_LAST, 0,
                   NULL, NULL, NULL, G_TYPE_NONE, 1, G_TYPE_BOOLEAN);
}

static void *get_proc_address(void *fn_ctx, const gchar *name) {
  GdkDisplay *display = gdk_display_get_default();

#ifdef GDK_WINDOWING_WAYLAND
  if (GDK_IS_WAYLAND_DISPLAY(display)) {
    return eglGetProcAddress(name);
  }
#endif
#ifdef GDK_WINDOWING_X11
  if (GDK_IS_X11_DISPLAY(display)) {
    return (void *)(intptr_t)glXGetProcAddressARB((const GLubyte *)name);
  }
#endif
#ifdef GDK_WINDOWING_WIN32
  if (GDK_IS_WIN32_DISPLAY(display)) {
    return wglGetProcAddress(name);
  }
#endif
  g_assert_not_reached();
  return NULL;
}

gboolean process_events(gpointer data) {
  struct _VpmVideoPlayerMpv *widget = (struct _VpmVideoPlayerMpv *)data;
  int done = 0;

  while (!done) {
    mpv_event *event = mpv_wait_event(widget->mpv_ctx->handle, 0);

    switch (event->event_id) {
    case MPV_EVENT_NONE:
      done = 1;
      break;

    case MPV_EVENT_LOG_MESSAGE:
      // printf("mpv log: %s\n", ((mpv_event_log_message *)event->data)->text);
      break;

    case MPV_EVENT_END_FILE:
      // TODO: might want to include reason (eof, error, redirect, etc.)
      g_signal_emit(widget, signals[SIGNAL_END_OF_FILE], 0);
      break;

    case MPV_EVENT_GET_PROPERTY_REPLY: {
      // TODO: We should probably check the error value here
      if (event->error < 0) {
        fprintf(stderr, "Error getting property reply\n");
        break;
      }
      __attribute__((fallthrough));
    }
    case MPV_EVENT_PROPERTY_CHANGE: {
      mpv_event_property *prop = (mpv_event_property *)event->data;

      switch (prop->format) {
      case MPV_FORMAT_DOUBLE:
        if (strcmp(prop->name, "duration") == 0) {
          g_signal_emit(widget, signals[SIGNAL_DURATION_UPDATED], 0,
                        *(double *)prop->data);
        } else if (strcmp(prop->name, "time-pos") == 0) {
          g_signal_emit(widget, signals[SIGNAL_POSITION_UPDATED], 0,
                        *(double *)prop->data);
        } else if (strcmp(prop->name, "ao-volume") == 0) {
          g_signal_emit(widget, signals[SIGNAL_VOLUME_UPDATED], 0,
                        *(double *)prop->data);
        }
        break;

      case MPV_FORMAT_NODE:
        if (strcmp(prop->name, "track-list") == 0) {
          mpv_node *data = prop->data;
          if (data->format == MPV_FORMAT_NODE_ARRAY) {
            GList *tracks = NULL;

            for (int i = 0; i < data->u.list->num; i++) {
              mpv_node trackNode = data->u.list->values[i];
              VpmTrack *track = track_node_to_track(trackNode);
              tracks = g_list_prepend(tracks, track);
            }

            VpmTrackList track_list = {.tracks = tracks};

            g_signal_emit(widget, signals[SIGNAL_TRACKS_UPDATED], 0,
                          &track_list);
          }
        }
        break;

      case MPV_FORMAT_FLAG: {
        gboolean val = *(int *)prop->data == 1;
        if (strcmp(prop->name, "ao-mute") == 0) {
          g_signal_emit(widget, signals[SIGNAL_MUTE_UPDATED], 0, val);
        } else if (strcmp(prop->name, "seeking") == 0) {
          g_signal_emit(widget, signals[SIGNAL_SEEKING], 0, val);
          // Ensure pause state stays correct after seeking
          mpv_get_property_async(widget->mpv_ctx->handle, 0, "pause",
                                 MPV_FORMAT_FLAG);
        } else if (strcmp(prop->name, "core-idle") == 0) {
          g_signal_emit(widget, signals[SIGNAL_CORE_IDLE], 0, val);
        } else if (strcmp(prop->name, "pause") == 0) {
          g_signal_emit(widget, signals[SIGNAL_PAUSE], 0, val);
        }
      }

      break;

      case MPV_FORMAT_STRING:
        if (strcmp(prop->name, "current-ao") == 0) {
          char *val = *(char **)prop->data;
          g_signal_emit(widget, signals[SIGNAL_CURRENT_AO], 0, val);
          // Once the audio output has been initialized, we want to emit update
          // signals for mute/volume to ensure we got the correct value
          mpv_get_property_async(widget->mpv_ctx->handle, 0, "ao-mute",
                                 MPV_FORMAT_FLAG);
          mpv_get_property_async(widget->mpv_ctx->handle, 0, "ao-volume",
                                 MPV_FORMAT_DOUBLE);
        }
        break;

      default:
        break;
      }

    } break;

    default:;
    }
  }

  return FALSE;
}

static void mpv_wakeup_callback(void *data) {
  g_idle_add_full(G_PRIORITY_HIGH_IDLE, process_events, data, NULL);
}

gboolean try_render_frame(gpointer data) {
  struct _VpmVideoPlayerMpv *widget = (struct _VpmVideoPlayerMpv *)data;
  if ((mpv_render_context_update(widget->mpv_ctx->render_context) &
       MPV_RENDER_UPDATE_FRAME)) {
    gtk_gl_area_queue_render(&widget->parent);
  }

  return FALSE;
}

static void mpv_update_callback(void *data) {
  g_idle_add_full(G_PRIORITY_HIGH_IDLE, try_render_frame, data, NULL);
}

static void resize(GtkGLArea *area, int width, int height, gpointer data) {
  struct MpvCtx *mpv_ctx = (struct MpvCtx *)data;
  mpv_ctx->width = width;
  mpv_ctx->height = height;
}

static void destroy(GtkGLArea *area, gpointer data) {
  struct MpvCtx *mpv_ctx = (struct MpvCtx *)data;
  mpv_render_context_set_update_callback(mpv_ctx->render_context, NULL, NULL);
  mpv_render_context_free(mpv_ctx->render_context);
  mpv_terminate_destroy(mpv_ctx->handle);
  free(mpv_ctx);
  mpv_ctx = NULL;
}

GtkWidget *vpm_video_player_mpv_new() {
  return g_object_new(vpm_video_player_mpv_get_type(), NULL);
}

gboolean render(GtkGLArea *widget, GdkGLContext *ctx, gpointer data) {
  struct MpvCtx *mpv_ctx = (struct MpvCtx *)data;

  gint fbo = -1;
  glGetIntegerv(GL_FRAMEBUFFER_BINDING, &fbo);

  mpv_opengl_fbo opengl_fbo = {fbo, mpv_ctx->width, mpv_ctx->height, 0};
  mpv_render_param params[] = {{MPV_RENDER_PARAM_OPENGL_FBO, &opengl_fbo},
                               {MPV_RENDER_PARAM_FLIP_Y, &(int){1}},
                               {MPV_RENDER_PARAM_INVALID, NULL}};

  mpv_render_context_render(mpv_ctx->render_context, params);

  return TRUE;
}

static void realize(GtkGLArea *area, gpointer data) {
  struct _VpmVideoPlayerMpv *widget = (struct _VpmVideoPlayerMpv *)data;

  gtk_gl_area_make_current(area);

  struct MpvCtx *mpv_ctx = malloc(sizeof *mpv_ctx);

  mpv_ctx->handle = mpv_create();
  mpv_set_property_string(mpv_ctx->handle, "vo", "libmpv");
  mpv_initialize(mpv_ctx->handle);
  mpv_request_log_messages(mpv_ctx->handle, "debug");

  widget->mpv_ctx = mpv_ctx;

  mpv_render_param params[] = {
      {MPV_RENDER_PARAM_API_TYPE, MPV_RENDER_API_TYPE_OPENGL},
      {MPV_RENDER_PARAM_OPENGL_INIT_PARAMS,
       &(mpv_opengl_init_params){
           .get_proc_address = get_proc_address,
       }},

      {MPV_RENDER_PARAM_ADVANCED_CONTROL, &(int){0}},
      {0}};

  mpv_render_context_create(&mpv_ctx->render_context, mpv_ctx->handle, params);

  mpv_set_wakeup_callback(mpv_ctx->handle, mpv_wakeup_callback, widget);
  mpv_render_context_set_update_callback(mpv_ctx->render_context,
                                         mpv_update_callback, widget);

  mpv_observe_property(mpv_ctx->handle, 0, "time-pos", MPV_FORMAT_DOUBLE);
  mpv_observe_property(mpv_ctx->handle, 0, "duration", MPV_FORMAT_DOUBLE);
  mpv_observe_property(mpv_ctx->handle, 0, "track-list", MPV_FORMAT_NODE);
  mpv_observe_property(mpv_ctx->handle, 0, "ao-volume", MPV_FORMAT_DOUBLE);
  mpv_observe_property(mpv_ctx->handle, 0, "ao-mute", MPV_FORMAT_FLAG);
  mpv_observe_property(mpv_ctx->handle, 0, "mute", MPV_FORMAT_FLAG);
  mpv_observe_property(mpv_ctx->handle, 0, "seeking", MPV_FORMAT_FLAG);
  mpv_observe_property(mpv_ctx->handle, 0, "core-idle", MPV_FORMAT_FLAG);
  mpv_observe_property(mpv_ctx->handle, 0, "current-ao", MPV_FORMAT_STRING);
  mpv_observe_property(mpv_ctx->handle, 0, "pause", MPV_FORMAT_FLAG);

  g_signal_connect(&widget->parent, "render", G_CALLBACK(render), mpv_ctx);
  g_signal_connect(&widget->parent, "resize", G_CALLBACK(resize), mpv_ctx);
  g_signal_connect(&widget->parent, "destroy", G_CALLBACK(destroy), mpv_ctx);

  g_signal_emit(widget, signals[SIGNAL_INITIALIZED], 0);
}

static void vpm_video_player_mpv_init(VpmVideoPlayerMpv *widget) {
  setlocale(LC_NUMERIC, "C");

  g_signal_connect(&widget->parent, "realize", G_CALLBACK(realize), widget);
}

void vpm_video_player_mpv_play_uri(VpmVideoPlayerMpv *self, const char *uri) {
  const char *cmd[] = {
      "loadfile",
      uri,
      NULL,
  };
  mpv_command(self->mpv_ctx->handle, cmd);
}

void vpm_video_player_mpv_play(VpmVideoPlayerMpv *self) {
  int val = FALSE;
  mpv_set_property(self->mpv_ctx->handle, "pause", MPV_FORMAT_FLAG, &val);
}

void vpm_video_player_mpv_pause(VpmVideoPlayerMpv *self) {
  int val = TRUE;
  mpv_set_property(self->mpv_ctx->handle, "pause", MPV_FORMAT_FLAG, &val);
}

void vpm_video_player_mpv_stop(VpmVideoPlayerMpv *self) {
  const char *cmd[] = {
      "stop",
      NULL,
  };
  mpv_command(self->mpv_ctx->handle, cmd);
}

double vpm_video_player_mpv_position(VpmVideoPlayerMpv *self) {
  double res;
  if (mpv_get_property(self->mpv_ctx->handle, "time-pos", MPV_FORMAT_DOUBLE,
                       &res) < 0) {
    fprintf(stderr, "Error getting time-pos.\n");
    return 0;
  }
  return res;
}

void vpm_video_player_mpv_seek_to(VpmVideoPlayerMpv *self, uint seconds) {
  gchar *val = g_strdup_printf("%i", seconds);
  const char *cmd[] = {"seek", val, "absolute", NULL};
  mpv_command(self->mpv_ctx->handle, cmd);
}

void vpm_video_player_mpv_seek_by(VpmVideoPlayerMpv *self, int seconds) {
  gchar *val = g_strdup_printf("%i", seconds);
  const char *cmd[] = {"seek", val, "relative", NULL};
  mpv_command(self->mpv_ctx->handle, cmd);
}

void vpm_video_player_mpv_frame_step_forwards(VpmVideoPlayerMpv *self) {
  const char *cmd[] = {"frame-step", NULL};
  mpv_command(self->mpv_ctx->handle, cmd);
}

void vpm_video_player_mpv_frame_step_backwards(VpmVideoPlayerMpv *self) {
  const char *cmd[] = {"frame-back-step", NULL};
  mpv_command(self->mpv_ctx->handle, cmd);
}

bool vpm_video_player_mpv_mute(VpmVideoPlayerMpv *self) {
  bool mute = false;
  if (mpv_get_property(self->mpv_ctx->handle, "ao-mute", MPV_FORMAT_FLAG,
                       &mute) < 0) {
    fprintf(stderr, "Error getting ao-mute.\n");
    return mute;
  }
  return mute;
}

void vpm_video_player_mpv_set_mute(VpmVideoPlayerMpv *self, bool mute) {
  int val = mute;
  mpv_set_property(self->mpv_ctx->handle, "ao-mute", MPV_FORMAT_FLAG, &val);
}

double vpm_video_player_mpv_volume(VpmVideoPlayerMpv *self) {
  double volume = 1;
  if (mpv_set_property(self->mpv_ctx->handle, "ao-volume", MPV_FORMAT_DOUBLE,
                       &volume) < 0) {
    fprintf(stderr, "Error getting ao-volume.\n");
  }
  return volume;
}

void vpm_video_player_mpv_set_volume(VpmVideoPlayerMpv *self, double volume) {
  mpv_set_property(self->mpv_ctx->handle, "ao-volume", MPV_FORMAT_DOUBLE,
                   &volume);
}

double vpm_video_player_mpv_playback_speed(VpmVideoPlayerMpv *self) {
  double playback_speed = 1;
  if (mpv_set_property(self->mpv_ctx->handle, "speed", MPV_FORMAT_DOUBLE,
                       &playback_speed) < 0) {
    fprintf(stderr, "Error getting speed.\n");
  }
  return playback_speed;
}

void vpm_video_player_mpv_set_playback_speed(VpmVideoPlayerMpv *self,
                                             double playback_speed) {
  int err = mpv_set_property(self->mpv_ctx->handle, "speed", MPV_FORMAT_DOUBLE,
                             &playback_speed);
  if (err < 0) {
    printf("Error setting playback speed: %d\n", err);
  }
}

int vpm_video_player_mpv_current_audio_track(VpmVideoPlayerMpv *self) {
  static int id;
  if (mpv_get_property(self->mpv_ctx->handle, "current-tracks/audio/id",
                       MPV_FORMAT_INT64, &id) < 0) {
    fprintf(stderr, "Error getting current-tracks/audio/id.\n");
    return -1;
  }
  return id;
}

void vpm_video_player_mpv_set_audio_track(VpmVideoPlayerMpv *self,
                                          uint audio_track_id) {
  uint64_t id = (uint64_t)audio_track_id;
  mpv_set_property(self->mpv_ctx->handle, "aid", MPV_FORMAT_INT64, &id);
}

int vpm_video_player_mpv_current_subtitle_track(VpmVideoPlayerMpv *self) {
  static int id;
  if (mpv_get_property(self->mpv_ctx->handle, "current-tracks/sub/id",
                       MPV_FORMAT_INT64, &id) < 0) {
    fprintf(stderr, "Error getting current-tracks/sub/id.\n");
    return -1;
  }
  return id;
}

void vpm_video_player_mpv_set_subtitle_track(VpmVideoPlayerMpv *self,
                                             uint subtitle_track_id) {
  uint64_t id = (uint64_t)subtitle_track_id;
  mpv_set_property(self->mpv_ctx->handle, "sid", MPV_FORMAT_INT64, &id);
}

void vpm_video_player_mpv_add_subtitle_track(VpmVideoPlayerMpv *self,
                                             const char *url,
                                             const char *title) {
  const char *cmd[] = {
      "sub-add", url, "auto", title, NULL,
  };
  if (mpv_command(self->mpv_ctx->handle, cmd) < 0) {
    printf("Error adding subtitle track %s", title);
  }
}

void vpm_video_player_mpv_set_subtitle_scale(VpmVideoPlayerMpv *self,
                                             double subtitle_scale) {
  mpv_set_option(self->mpv_ctx->handle, "sub-scale", MPV_FORMAT_DOUBLE,
                 &subtitle_scale);
}

void vpm_video_player_mpv_set_subtitle_colour(VpmVideoPlayerMpv *self,
                                              char *colour) {
  mpv_set_option(self->mpv_ctx->handle, "sub-color", MPV_FORMAT_STRING,
                 &colour);
}

void vpm_video_player_mpv_set_subtitle_background_colour(
    VpmVideoPlayerMpv *self, char *colour) {
  mpv_set_option(self->mpv_ctx->handle, "sub-back-color", MPV_FORMAT_STRING,
                 &colour);
}

void vpm_video_player_mpv_set_subtitle_position(VpmVideoPlayerMpv *self,
                                                uint position) {
  uint64_t position_int = (uint64_t)position;
  mpv_set_option(self->mpv_ctx->handle, "sub-pos", MPV_FORMAT_INT64,
                 &position_int);
}

void vpm_video_player_mpv_set_subtitle_font_family(VpmVideoPlayerMpv *self,
                                                   char *family) {
  int err = mpv_set_option(self->mpv_ctx->handle, "sub-font", MPV_FORMAT_STRING,
                           &family);
  if (err < 0) {
    printf("Error setting sub-font: %d\n", err);
  }
}

void vpm_video_player_mpv_set_subtitle_font_size(VpmVideoPlayerMpv *self,
                                                 uint size) {
  uint64_t size_int = (uint64_t)size;
  int err = mpv_set_option(self->mpv_ctx->handle, "sub-font-size",
                           MPV_FORMAT_INT64, &size_int);
  if (err < 0) {
    printf("Error setting sub-font-size: %d\n", err);
  }
}

void vpm_video_player_mpv_set_subtitle_font_bold(VpmVideoPlayerMpv *self,
                                                 bool bold) {
  int val = bold;
  mpv_set_property(self->mpv_ctx->handle, "sub-bold", MPV_FORMAT_FLAG, &val);
}

void vpm_video_player_mpv_set_subtitle_font_italic(VpmVideoPlayerMpv *self,
                                                   bool italic) {
  int val = italic;
  mpv_set_property(self->mpv_ctx->handle, "sub-italic", MPV_FORMAT_FLAG, &val);
}
