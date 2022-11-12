#ifndef __BINDINGS_TRINITY_MODULE_H
#define __BINDINGS_TRINITY_MODULE_H
#ifdef __cplusplus
extern "C" {
#endif

//#include <stdlib.h>
#include <string.h>
#include <stdint.h>
#include <stdbool.h>

typedef struct {
  char*ptr;
  size_t len;
} trinity_module_string_t;

typedef struct {
  trinity_module_string_t content;
  trinity_module_string_t to;
} trinity_module_message_t;

typedef struct {
  bool is_some;
  trinity_module_string_t val;
} trinity_module_option_string_t;

typedef struct {
  trinity_module_message_t *ptr;
  size_t len;
} trinity_module_list_message_t;

// Imported Functions from `sys`

__attribute__((import_module("sys"), import_name("rand-u64")))
int64_t __wasm_import_sys_rand_u64(void);
uint64_t sys_rand_u64(void);

// Imported Functions from `log`

__attribute__((import_module("log"), import_name("trace")))
void __wasm_import_log_trace(int32_t, int32_t);
void log_trace(trinity_module_string_t *s);

__attribute__((import_module("log"), import_name("debug")))
void __wasm_import_log_debug(int32_t, int32_t);
void log_debug(trinity_module_string_t *s);

__attribute__((import_module("log"), import_name("info")))
void __wasm_import_log_info(int32_t, int32_t);
void log_info(trinity_module_string_t *s);

__attribute__((import_module("log"), import_name("warn")))
void __wasm_import_log_warn(int32_t, int32_t);
void log_warn(trinity_module_string_t *s);

__attribute__((import_module("log"), import_name("error")))
void __wasm_import_log_error(int32_t, int32_t);
void log_error(trinity_module_string_t *s);

// Exported Functions from `trinity-module`
void trinity_module_init(void);
void trinity_module_help(trinity_module_option_string_t *topic, trinity_module_string_t *ret);
void trinity_module_admin(trinity_module_string_t *cmd, trinity_module_string_t *author_id, trinity_module_list_message_t *ret);
void trinity_module_on_msg(trinity_module_string_t *content, trinity_module_string_t *author_id, trinity_module_string_t *author_name, trinity_module_string_t *room, trinity_module_list_message_t *ret);

// Helper Functions

void trinity_module_message_free(trinity_module_message_t *ptr);
void trinity_module_option_string_free(trinity_module_option_string_t *ptr);
void trinity_module_list_message_free(trinity_module_list_message_t *ptr);
void trinity_module_string_set(trinity_module_string_t *ret, const char*s);
void trinity_module_string_dup(trinity_module_string_t *ret, const char*s);
void trinity_module_string_free(trinity_module_string_t *ret);

#ifdef __cplusplus
}
#endif
#endif
