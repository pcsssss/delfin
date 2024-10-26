#pragma once

#include "video-player-mpv/track.h"
#include <gtk/gtk.h>
#include <stdbool.h>
#include <sys/types.h>

G_BEGIN_DECLS

G_DECLARE_FINAL_TYPE(VpmVideoPlayerMpv, vpm_video_player_mpv, VPM,
                     VIDEO_PLAYER_MPV, GtkGLArea)

GtkWidget *vpm_video_player_mpv_new();

int vpm_video_player_mpv_set_property_string(VpmVideoPlayerMpv *self,
                                             const char *name,
                                             const char *value);

void vpm_video_player_mpv_play_uri(VpmVideoPlayerMpv *self, const char *uri);
void vpm_video_player_mpv_play(VpmVideoPlayerMpv *self);
void vpm_video_player_mpv_pause(VpmVideoPlayerMpv *self);
void vpm_video_player_mpv_stop(VpmVideoPlayerMpv *self);

double vpm_video_player_mpv_position(VpmVideoPlayerMpv *self);
void vpm_video_player_mpv_seek_to(VpmVideoPlayerMpv *self, uint seconds);
void vpm_video_player_mpv_seek_by(VpmVideoPlayerMpv *self, int seconds);
void vpm_video_player_mpv_frame_step_forwards(VpmVideoPlayerMpv *self);
void vpm_video_player_mpv_frame_step_backwards(VpmVideoPlayerMpv *self);

bool vpm_video_player_mpv_mute(VpmVideoPlayerMpv *self);
void vpm_video_player_mpv_set_mute(VpmVideoPlayerMpv *self, bool mute);
double vpm_video_player_mpv_volume(VpmVideoPlayerMpv *self);
void vpm_video_player_mpv_set_volume(VpmVideoPlayerMpv *self, double volume);

double vpm_video_player_mpv_playback_speed(VpmVideoPlayerMpv *self);
void vpm_video_player_mpv_set_playback_speed(VpmVideoPlayerMpv *self,
                                             double playback_speed);

int vpm_video_player_mpv_current_audio_track(VpmVideoPlayerMpv *self);
void vpm_video_player_mpv_set_audio_track(VpmVideoPlayerMpv *self,
                                          uint audio_track_id);

int vpm_video_player_mpv_current_subtitle_track(VpmVideoPlayerMpv *self);
void vpm_video_player_mpv_set_subtitle_track(VpmVideoPlayerMpv *self,
                                             uint subtitle_track_id);
void vpm_video_player_mpv_add_subtitle_track(VpmVideoPlayerMpv *self,
                                             const char *url,
                                             const char *title);

void vpm_video_player_mpv_set_subtitle_scale(VpmVideoPlayerMpv *self,
                                             double subtitle_scale);
void vpm_video_player_mpv_set_subtitle_colour(VpmVideoPlayerMpv *self,
                                              char *colour);
void vpm_video_player_mpv_set_subtitle_background_colour(
    VpmVideoPlayerMpv *self, char *colour);
void vpm_video_player_mpv_set_subtitle_position(VpmVideoPlayerMpv *self,
                                                uint position);
void vpm_video_player_mpv_set_subtitle_font_family(VpmVideoPlayerMpv *self,
                                                   char *family);
void vpm_video_player_mpv_set_subtitle_font_size(VpmVideoPlayerMpv *self,
                                                 uint size);
void vpm_video_player_mpv_set_subtitle_font_bold(VpmVideoPlayerMpv *self,
                                                 bool bold);
void vpm_video_player_mpv_set_subtitle_font_italic(VpmVideoPlayerMpv *self,
                                                   bool italic);

G_END_DECLS
