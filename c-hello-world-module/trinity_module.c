#include "trinity_module.h"

__attribute__((weak, export_name("cabi_post_help")))
void __wasm_export_trinity_module_help_post_return(int32_t arg0) {
  if ((*((int32_t*) (arg0 + 4))) > 0) {
    free((void*) (*((int32_t*) (arg0 + 0))));
  }
}
__attribute__((weak, export_name("cabi_post_admin")))
void __wasm_export_trinity_module_admin_post_return(int32_t arg0) {
  int32_t ptr = *((int32_t*) (arg0 + 0));
  int32_t len = *((int32_t*) (arg0 + 4));
  for (int32_t i = 0; i < len; i++) {
    int32_t base = ptr + i * 16;
    (void) base;
    if ((*((int32_t*) (base + 4))) > 0) {
      free((void*) (*((int32_t*) (base + 0))));
    }
    if ((*((int32_t*) (base + 12))) > 0) {
      free((void*) (*((int32_t*) (base + 8))));
    }
  }
  if (len > 0) {
    free((void*) (ptr));
  }
}
__attribute__((weak, export_name("cabi_post_on-msg")))
void __wasm_export_trinity_module_on_msg_post_return(int32_t arg0) {
  int32_t ptr = *((int32_t*) (arg0 + 0));
  int32_t len = *((int32_t*) (arg0 + 4));
  for (int32_t i = 0; i < len; i++) {
    int32_t base = ptr + i * 16;
    (void) base;
    if ((*((int32_t*) (base + 4))) > 0) {
      free((void*) (*((int32_t*) (base + 0))));
    }
    if ((*((int32_t*) (base + 12))) > 0) {
      free((void*) (*((int32_t*) (base + 8))));
    }
  }
  if (len > 0) {
    free((void*) (ptr));
  }
}

__attribute__((weak, export_name("cabi_realloc")))
void *cabi_realloc(void *ptr, size_t orig_size, size_t org_align, size_t new_size) {
  void *ret = realloc(ptr, new_size);
  if (!ret) abort();
  return ret;
}

// Helper Functions

void trinity_module_message_free(trinity_module_message_t *ptr) {
  trinity_module_string_free(&ptr->content);
  trinity_module_string_free(&ptr->to);
}

void trinity_module_option_string_free(trinity_module_option_string_t *ptr) {
  if (ptr->is_some) {
    trinity_module_string_free(&ptr->val);
  }
}

void trinity_module_list_message_free(trinity_module_list_message_t *ptr) {
  for (size_t i = 0; i < ptr->len; i++) {
    trinity_module_message_free(&ptr->ptr[i]);
  }
  if (ptr->len > 0) {
    free(ptr->ptr);
  }
}

void trinity_module_string_set(trinity_module_string_t *ret, const char*s) {
  ret->ptr = (char*) s;
  ret->len = strlen(s);
}

void trinity_module_string_dup(trinity_module_string_t *ret, const char*s) {
  ret->len = strlen(s);
  ret->ptr = cabi_realloc(NULL, 0, 1, ret->len * 1);
  memcpy(ret->ptr, s, ret->len * 1);
}

void trinity_module_string_free(trinity_module_string_t *ret) {
  if (ret->len > 0) {
    free(ret->ptr);
  }
  ret->ptr = NULL;
  ret->len = 0;
}

// Component Adapters

__attribute__((aligned(4)))
static uint8_t RET_AREA[8];

uint64_t sys_rand_u64(void) {
  int64_t ret = __wasm_import_sys_rand_u64();
  return (uint64_t) (ret);
}

void log_trace(trinity_module_string_t *s) {
  __wasm_import_log_trace((int32_t) (*s).ptr, (int32_t) (*s).len);
}

void log_debug(trinity_module_string_t *s) {
  __wasm_import_log_debug((int32_t) (*s).ptr, (int32_t) (*s).len);
}

void log_info(trinity_module_string_t *s) {
  __wasm_import_log_info((int32_t) (*s).ptr, (int32_t) (*s).len);
}

void log_warn(trinity_module_string_t *s) {
  __wasm_import_log_warn((int32_t) (*s).ptr, (int32_t) (*s).len);
}

void log_error(trinity_module_string_t *s) {
  __wasm_import_log_error((int32_t) (*s).ptr, (int32_t) (*s).len);
}

__attribute__((export_name("init")))
void __wasm_export_trinity_module_init(void) {
  trinity_module_init();
}

__attribute__((export_name("help")))
int32_t __wasm_export_trinity_module_help(int32_t arg, int32_t arg0, int32_t arg1) {
  trinity_module_option_string_t option;
  switch (arg) {
    case 0: {
      option.is_some = false;
      break;
    }
    case 1: {
      option.is_some = true;
      option.val = (trinity_module_string_t) { (char*)(arg0), (size_t)(arg1) };
      break;
    }
  }
  trinity_module_option_string_t arg2 = option;
  trinity_module_string_t ret;
  trinity_module_help(&arg2, &ret);
  int32_t ptr = (int32_t) &RET_AREA;
  *((int32_t*)(ptr + 4)) = (int32_t) (ret).len;
  *((int32_t*)(ptr + 0)) = (int32_t) (ret).ptr;
  return ptr;
}

__attribute__((export_name("admin")))
int32_t __wasm_export_trinity_module_admin(int32_t arg, int32_t arg0, int32_t arg1, int32_t arg2) {
  trinity_module_string_t arg3 = (trinity_module_string_t) { (char*)(arg), (size_t)(arg0) };
  trinity_module_string_t arg4 = (trinity_module_string_t) { (char*)(arg1), (size_t)(arg2) };
  trinity_module_list_message_t ret;
  trinity_module_admin(&arg3, &arg4, &ret);
  int32_t ptr = (int32_t) &RET_AREA;
  *((int32_t*)(ptr + 4)) = (int32_t) (ret).len;
  *((int32_t*)(ptr + 0)) = (int32_t) (ret).ptr;
  return ptr;
}

__attribute__((export_name("on-msg")))
int32_t __wasm_export_trinity_module_on_msg(int32_t arg, int32_t arg0, int32_t arg1, int32_t arg2, int32_t arg3, int32_t arg4, int32_t arg5, int32_t arg6) {
  trinity_module_string_t arg7 = (trinity_module_string_t) { (char*)(arg), (size_t)(arg0) };
  trinity_module_string_t arg8 = (trinity_module_string_t) { (char*)(arg1), (size_t)(arg2) };
  trinity_module_string_t arg9 = (trinity_module_string_t) { (char*)(arg3), (size_t)(arg4) };
  trinity_module_string_t arg10 = (trinity_module_string_t) { (char*)(arg5), (size_t)(arg6) };
  trinity_module_list_message_t ret;
  trinity_module_on_msg(&arg7, &arg8, &arg9, &arg10, &ret);
  int32_t ptr = (int32_t) &RET_AREA;
  *((int32_t*)(ptr + 4)) = (int32_t) (ret).len;
  *((int32_t*)(ptr + 0)) = (int32_t) (ret).ptr;
  return ptr;
}

extern void __component_type_object_force_link_trinity_module(void);
void __component_type_object_force_link_trinity_module_public_use_in_this_compilation_unit(void) {
  __component_type_object_force_link_trinity_module();
}
