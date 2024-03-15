#include <stdarg.h>
#include <stdbool.h>
#include <stdint.h>
#include <stdlib.h>

void c_start(const char *server);

void c_toggle(void);

void c_stop(void);

unsigned short c_get_state(void);

char *c_get_metadata_artist(void);

char *c_get_metadata_album(void);

char *c_get_metadata_title(void);

float *c_get_metadata_length(void);
