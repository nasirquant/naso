; Injections for Naso language

; Inject Naso into markdown code blocks
((fenced_code_block
  info_string: (language) @_lang
  content: (code_fence_content) @injection.content)
 (#eq? @_lang "naso"))

; Inject Naso into string literals with specific prefixes
(string_literal
  (#match? @_prefix "^q#")
  @injection.content)
