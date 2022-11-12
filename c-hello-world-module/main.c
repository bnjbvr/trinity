#include "trinity_module.h"

void trinity_module_init(void) {
  trinity_module_string_t str = {.ptr = "hello", .len = strlen("hello")};
  log_debug(&str);
}

void trinity_module_help(trinity_module_option_string_t *topic,
                         trinity_module_string_t *ret) {
  trinity_module_string_set(ret,
                            "a simple module showing how trinity works in C");
}

void trinity_module_admin(trinity_module_string_t *cmd,
                          trinity_module_string_t *author_id,
                          trinity_module_list_message_t *ret) {
  ret->len = 0;
}

void trinity_module_on_msg(trinity_module_string_t *content,
                           trinity_module_string_t *author_id,
                           trinity_module_string_t *author_name,
                           trinity_module_string_t *room,
                           trinity_module_list_message_t *ret) {
  trinity_module_message_t *msg = malloc(sizeof(trinity_module_message_t));
  ret->ptr = msg;
  ret->len = 1;

  msg->to.ptr = author_id->ptr;
  msg->to.len = author_id->len;

  char *ptr =
      malloc(sizeof(char) * (strlen(author_id->ptr) + strlen("Hello, !")));

  // ogod this is dirty. what it takes to not use wasi 🙈
  strcpy(ptr, "Hello, ");
  strcpy(ptr + strlen("Hello, "), author_id->ptr);
  strcpy(ptr + strlen("Hello, ") + strlen(author_id->ptr), "!");

  msg->content.ptr = ptr;
  msg->content.len = strlen(ptr);
}
