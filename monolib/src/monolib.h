#include <stdarg.h>
#include <stdbool.h>
#include <stdint.h>
#include <stdlib.h>

typedef struct CSettings {
  /**
   * See lonelyradio_types for numeric representation -> Encoder
   */
  uint8_t encoder;
  int32_t cover;
} CSettings;

typedef struct CImageJpeg {
  uint32_t length;
  uint8_t *bytes;
} CImageJpeg;

/**
 * Starts audio playback using rodio
 * Play without playlist => playlist = ""
 */
void c_start(const char *server, struct CSettings settings, const char *playlist);

/**
 * Playlists separated by '\n'
 */
char *c_list_playlists(const char *server);

void c_toggle(void);

void c_stop(void);

char c_get_state(void);

char *c_get_metadata_artist(void);

char *c_get_metadata_album(void);

char *c_get_metadata_title(void);

float c_get_metadata_length(void);

/**
 * # Safety
 * Manually deallocate returned memory after use
 */
struct CImageJpeg c_get_cover_jpeg(void);

/**
 * # Safety
 * None
 */
void c_drop(uint8_t *ptr, uintptr_t count);
