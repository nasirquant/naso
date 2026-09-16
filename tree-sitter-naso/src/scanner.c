#include <tree_sitter/parser.h>

enum TokenType {
  // Token types would be generated here
};

void *tree_sitter_naso_external_scanner_create() { return NULL; }
void tree_sitter_naso_external_scanner_destroy(void *payload) {}
bool tree_sitter_naso_external_scanner_scan(void *payload, TSLexer *lexer, const bool *valid_symbols) { return false; }
unsigned tree_sitter_naso_external_scanner_serialize(void *payload, char *buffer) { return 0; }
void tree_sitter_naso_external_scanner_deserialize(void *payload, const char *buffer, unsigned length) {}
