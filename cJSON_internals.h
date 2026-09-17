/*
  Declarations for cJSON internals that the Unity unit tests call.
  These were previously reached by `#include "../cJSON.c"`; the
  implementation now lives in the Rust C-ABI library (libcjson).
*/
#ifndef CJSON_INTERNALS_H
#define CJSON_INTERNALS_H

#include "cJSON.h"
#include <stddef.h>

#ifdef __cplusplus
extern "C" {
#endif

typedef struct internal_hooks
{
    void *(CJSON_CDECL *allocate)(size_t size);
    void (CJSON_CDECL *deallocate)(void *pointer);
    void *(CJSON_CDECL *reallocate)(void *pointer, size_t size);
} internal_hooks;

typedef struct
{
    const unsigned char *content;
    size_t length;
    size_t offset;
    size_t depth;
    internal_hooks hooks;
} parse_buffer;

typedef struct
{
    unsigned char *buffer;
    size_t length;
    size_t offset;
    size_t depth;
    cJSON_bool noalloc;
    cJSON_bool format;
    internal_hooks hooks;
} printbuffer;

#define can_read(buffer, size) ((buffer != NULL) && (((buffer)->offset + size) <= (buffer)->length))
#define can_access_at_index(buffer, index) ((buffer != NULL) && (((buffer)->offset + index) < (buffer)->length))
#define cannot_access_at_index(buffer, index) (!can_access_at_index(buffer, index))
#define buffer_at_offset(buffer) ((buffer)->content + (buffer)->offset)

#ifndef true
#define true ((cJSON_bool)1)
#endif
#ifndef false
#define false ((cJSON_bool)0)
#endif

extern internal_hooks global_hooks;

cJSON_bool parse_number(cJSON * const item, parse_buffer * const input_buffer);
cJSON_bool parse_string(cJSON * const item, parse_buffer * const input_buffer);
cJSON_bool parse_array(cJSON * const item, parse_buffer * const input_buffer);
cJSON_bool parse_object(cJSON * const item, parse_buffer * const input_buffer);
cJSON_bool parse_value(cJSON * const item, parse_buffer * const input_buffer);
unsigned parse_hex4(const unsigned char * const input);

cJSON_bool print_number(const cJSON * const item, printbuffer * const output_buffer);
cJSON_bool print_string(const cJSON * const item, printbuffer * const p);
cJSON_bool print_string_ptr(const unsigned char * const input, printbuffer * const output_buffer);
cJSON_bool print_array(const cJSON * const item, printbuffer * const output_buffer);
cJSON_bool print_object(const cJSON * const item, printbuffer * const output_buffer);
cJSON_bool print_value(const cJSON * const item, printbuffer * const output_buffer);

unsigned char *ensure(printbuffer * const p, size_t needed);
parse_buffer *skip_utf8_bom(parse_buffer * const buffer);
cJSON_bool add_item_to_array(cJSON *array, cJSON *item);
unsigned char *cJSON_strdup(const unsigned char *string, const internal_hooks * const hooks);
cJSON_bool compare_double(double a, double b);

#ifdef __cplusplus
}
#endif

#endif /* CJSON_INTERNALS_H */
