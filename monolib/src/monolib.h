#include <stdarg.h>
#include <stdbool.h>
#include <stddef.h>
#include <stdint.h>
#include <stdlib.h>

typedef struct CImageJpeg {
  uint32_t length;
  uint8_t *bytes;
} CImageJpeg;

typedef struct CSettings {
  /**
   * See lonelyradio_types -> Encoder
   */
  uint8_t encoder;
  int32_t cover;
} CSettings;

void c_drop(uint8_t *ptr, size_t count);

/**
 * # Safety
 * Manually deallocate returned memory after use
 */
struct CImageJpeg c_get_cover_jpeg(void);

char *c_get_metadata_album(void);

char *c_get_metadata_artist(void);

float c_get_metadata_length(void);

char *c_get_metadata_title(void);

char c_get_state(void);

void c_start(const char *server, struct CSettings settings);

void c_stop(void);

void c_toggle(void);
