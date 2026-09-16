{
  "targets": [
    {
      "target_name": "tree_sitter_naso",
      "sources": [
        "src/parser.c",
        "src/scanner.c"
      ],
      "include_dirs": [
        "src"
      ],
      "cflags": [
        "-std=c11",
        "-Wall",
        "-Wextra",
        "-Wno-unused-parameter"
      ],
      "conditions": [
        ["OS==\"win\"", {
          "cflags": [
            "/std:c11",
            "/Wall",
            "/WX"
          ]
        }]
      ]
    }
  ]
}
