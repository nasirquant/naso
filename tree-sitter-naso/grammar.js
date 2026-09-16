/// <reference types="tree-sitter-cli/dsl" />
// @ts-check

/**
 * Tree-sitter grammar for the Naso programming language
 * Naso: A quantum-typed systems programming language with QTT linearity
 */

module.exports = grammar({
  name: 'naso',

  extras: $ => [
    $.comment,
    /\s/,
  ],

  conflicts: $ => [
    [$.type_annotation, $.generic_type],
    [$.quantity_annotation, $.array_type],
    [$.function_signature, $.function_declaration],
    [$.quantum_intrinsic, $.function_call],
    [$.tensor_operation, $.function_call],
    [$.polyhedral_loop, $.for_loop],
    [$.range_expression, $._expression],
    [$.range_expression, $.function_call],
    [$.generic_arguments, $._expression],
    [$._statement, $.block_expression],
    [$.unary_expression, $.binary_expression],
    [$.unary_expression, $.cast_expression],
    [$.unary_expression, $.as_expression],
    [$.binary_expression, $.cast_expression],
    [$.binary_expression, $.as_expression],
    [$.cast_expression, $.as_expression],
    [$.struct_literal, $._expression],
    [$.struct_literal, $.range_expression],
    [$.struct_literal, $.if_expression],
    [$.parenthesized_expression, $.tuple_literal],
    [$.block_expression, $.lambda_expression],
    [$.lambda_expression, $.index_access],
    [$.lambda_expression, $.range_expression],
    [$.method_call, $.lambda_expression],
    [$.field_access, $.lambda_expression],
    [$._type, $.generic_type],
    [$.range_expression, $.index_access],
    [$.if_statement, $.if_expression],
    [$.index_access, $.forall_expression],
    [$.index_access, $.tile_expression],
    [$.range_expression, $.forall_expression],
    [$.range_expression, $.tile_expression],
    [$.forall_loop, $.block_expression],
    [$.match_statement, $.match_expression],
    [$.pattern, $.unit_literal],
    [$.tile_loop, $.block_expression],
    [$.match_arm, $.block_expression],
  ],

  word: $ => $.identifier,

  supertypes: $ => [
    $._expression,
    $._statement,
    $._type,
    $._declaration,
    $.quantum_intrinsic,
    $.tensor_operation,
    $.polyhedral_expression,
  ],

  rules: {
    // =========================================================================
    // ENTRY POINT
    // =========================================================================
    source_file: $ => repeat($._declaration),

    // =========================================================================
    // DECLARATIONS
    // =========================================================================
    _declaration: $ => choice(
      $.module_declaration,
      $.import_declaration,
      $.function_declaration,
      $.struct_declaration,
      $.enum_declaration,
      $.type_alias_declaration,
      $.const_declaration,
      $.static_declaration,
    ),

    module_declaration: $ => seq(
      'module',
      $.identifier,
      '{',
      repeat($._declaration),
      '}'
    ),

    import_declaration: $ => seq(
      'import',
      choice(
        $.identifier,
        seq('{', commaSep($.identifier), '}'),
        seq($.identifier, 'as', $.identifier)
      ),
      optional(seq('from', $.string_literal)),
      ';'
    ),

    function_declaration: $ => seq(
      optional($.visibility_modifier),
      'fn',
      $.identifier,
      optional($.generic_parameters),
      $.function_signature,
      choice(
        $.block,
        ';'
      )
    ),

    function_signature: $ => seq(
      '(',
      commaSep($.parameter),
      ')',
      optional(seq('->', $.type_annotation))
    ),

    parameter: $ => choice(
      seq(optional('inout'), $.identifier, ':', $.type_annotation),
      seq(optional('inout'), $.identifier, ':', $.quantity_annotation, $.type_annotation),
      seq('...', $.identifier, ':', $.type_annotation)
    ),

    generic_parameters: $ => seq(
      '<',
      commaSep($.generic_parameter),
      '>'
    ),

    generic_parameter: $ => choice(
      $.identifier,
      seq($.identifier, ':', $.constraint)
    ),

    constraint: $ => choice(
      $.identifier,
      seq($.identifier, '+', $.identifier)
    ),

    struct_declaration: $ => seq(
      optional($.visibility_modifier),
      'struct',
      $.identifier,
      optional($.generic_parameters),
      '{',
      commaSep($.field_declaration),
      '}'
    ),

    field_declaration: $ => seq(
      optional($.visibility_modifier),
      $.identifier,
      ':',
      $.type_annotation,
      optional(seq('=', $._expression))
    ),

    enum_declaration: $ => seq(
      optional($.visibility_modifier),
      'enum',
      $.identifier,
      optional($.generic_parameters),
      '{',
      commaSep($.enum_variant),
      '}'
    ),

    enum_variant: $ => seq(
      $.identifier,
      optional(seq('(', commaSep($.type_annotation), ')'))
    ),

    type_alias_declaration: $ => seq(
      optional($.visibility_modifier),
      'type',
      $.identifier,
      optional($.generic_parameters),
      '=',
      $.type_annotation,
      ';'
    ),

    const_declaration: $ => seq(
      optional($.visibility_modifier),
      'const',
      $.identifier,
      optional(seq(':', $.type_annotation)),
      '=',
      $._expression,
      ';'
    ),

    static_declaration: $ => seq(
      optional($.visibility_modifier),
      'static',
      'mut',
      $.identifier,
      ':',
      $.type_annotation,
      '=',
      $._expression,
      ';'
    ),

    visibility_modifier: $ => choice('pub', 'pub(crate)', 'pub(super)', 'pub(in', $.identifier, ')'),

    // =========================================================================
    // TYPES & QUANTITY ANNOTATIONS (QTT)
    // =========================================================================
    _type: $ => choice(
      $.primitive_type,
      $.quantity_annotation,
      $.generic_type,
      $.function_type,
      $.tuple_type,
      $.array_type,
      $.reference_type,
      $.inout_type,
      $.identifier,
    ),

    primitive_type: $ => choice(
      'bool',
      'u8', 'u16', 'u32', 'u64', 'u128', 'usize',
      'i8', 'i16', 'i32', 'i64', 'i128', 'isize',
      'f16', 'f32', 'f64',
      'char',
      'str',
      'never',
      'unit',
      'qubit',
      'qir::qubit',
    ),

    // QTT Quantity Annotations: [0], [1], [*], [N]
    quantity_annotation: $ => seq(
      '[',
      choice(
        '0',
        '1',
        '*',
        $.identifier,  // [N] for bounded quantities
        $.number_literal
      ),
      ']',
      $.type_annotation
    ),

    generic_type: $ => seq(
      $.identifier,
      '<',
      commaSep($.type_annotation),
      '>'
    ),

    function_type: $ => seq(
      'fn',
      optional($.generic_parameters),
      '(',
      commaSep($.type_annotation),
      ')',
      optional(seq('->', $.type_annotation))
    ),

    tuple_type: $ => seq(
      '(',
      commaSep1($.type_annotation),
      ')'
    ),

    array_type: $ => seq(
      '[',
      $.type_annotation,
      ';',
      $.number_literal,
      ']'
    ),

    reference_type: $ => seq(
      '&',
      optional($.lifetime),
      optional('mut'),
      $.type_annotation
    ),

    inout_type: $ => seq(
      'inout',
      $.type_annotation
    ),

    lifetime: $ => seq('\'', $.identifier),

    type_annotation: $ => $._type,

    // =========================================================================
    // STATEMENTS
    // =========================================================================
    _statement: $ => choice(
      $.let_statement,
      $.assignment_statement,
      $.expression_statement,
      $.return_statement,
      $.break_statement,
      $.continue_statement,
      $.if_statement,
      $.while_statement,
      $.for_loop,
      $.polyhedral_loop,
      $.match_statement,
      $.block,
      $.uncomputation_statement,
      $.defer_statement,
    ),

    let_statement: $ => seq(
      'let',
      optional('mut'),
      choice(
        $.identifier,
        seq('(', commaSep($.identifier), ')')
      ),
      optional(seq(':', $.type_annotation)),
      optional(seq('=', $._expression)),
      ';'
    ),

    assignment_statement: $ => seq(
      $._expression,
      choice('=', '+=', '-=', '*=', '/=', '%=', '&=', '|=', '^=', '<<=', '>>='),
      $._expression,
      ';'
    ),

    expression_statement: $ => seq($._expression, ';'),

    return_statement: $ => seq('return', optional($._expression), ';'),

    break_statement: $ => seq('break', optional($.label), ';'),

    continue_statement: $ => seq('continue', optional($.label), ';'),

    label: $ => seq('\'', $.identifier),

    if_statement: $ => seq(
      'if',
      $._expression,
      $.block,
      optional(seq('else', choice($.block, $.if_statement)))
    ),

    while_statement: $ => seq(
      optional($.label),
      'while',
      $._expression,
      $.block
    ),

    for_loop: $ => seq(
      optional($.label),
      'for',
      $.identifier,
      'in',
      $._expression,
      $.block
    ),

    // Polyhedral loop constructs: forall, tile, fuse
    polyhedral_loop: $ => choice(
      $.forall_loop,
      $.tile_loop,
      $.fuse_loop,
    ),

    forall_loop: $ => seq(
      'forall',
      optional($.schedule_annotation),
      '(',
      commaSep($.forall_binding),
      ')',
      $.block
    ),

    forall_binding: $ => seq(
      $.identifier,
      'in',
      $.range_expression
    ),

    range_expression: $ => choice(
      prec.right(seq($._expression, '..', $._expression)),
      prec.right(seq($._expression, '..=', $._expression)),
      $.identifier,
    ),

    tile_loop: $ => seq(
      'tile',
      $.schedule_annotation,
      '(',
      commaSep($.identifier),
      ')',
      $.block
    ),

    fuse_loop: $ => seq(
      'fuse',
      '(',
      commaSep($.identifier),
      ')',
      $.block
    ),

    schedule_annotation: $ => seq(
      '#[schedule(',
      commaSep($.schedule_parameter),
      ')]'
    ),

    schedule_parameter: $ => choice(
      seq($.identifier, '=', $._expression),
      $.identifier,
    ),

    match_statement: $ => seq(
      'match',
      $._expression,
      '{',
      commaSep($.match_arm),
      '}'
    ),

    match_arm: $ => seq(
      $.pattern,
      optional(seq('if', $._expression)),
      '=>',
      choice($._expression, $.block),
      ','
    ),

    pattern: $ => choice(
      $.identifier,
      $.literal,
      seq('(', commaSep($.pattern), ')'),
      seq($.identifier, '(', commaSep($.pattern), ')'),
      seq('..', $.identifier),
      '_',
    ),

    block: $ => seq(
      '{',
      repeat($._statement),
      '}'
    ),

    uncomputation_statement: $ => seq(
      'uncompute',
      $._expression,
      ';'
    ),

    defer_statement: $ => seq(
      'defer',
      $.block
    ),

    // =========================================================================
    // EXPRESSIONS
    // =========================================================================
    _expression: $ => choice(
      $.literal,
      $.identifier,
      $.parenthesized_expression,
      $.function_call,
      $.method_call,
      $.field_access,
      $.index_access,
      $.binary_expression,
      $.unary_expression,
      $.cast_expression,
      $.as_expression,
      $.block_expression,
      $.if_expression,
      $.match_expression,
      $.lambda_expression,
      $.quantum_intrinsic,
      $.tensor_operation,
      $.polyhedral_expression,
      $.array_literal,
      $.tuple_literal,
      $.struct_literal,
      $.range_expression,
    ),

    parenthesized_expression: $ => seq('(', $._expression, ')'),

    function_call: $ => prec(1, seq(
      $.identifier,
      optional($.generic_arguments),
      '(',
      commaSep($._expression),
      ')'
    )),

    method_call: $ => prec(2, seq(
      $._expression,
      '.',
      $.identifier,
      optional($.generic_arguments),
      '(',
      commaSep($._expression),
      ')'
    )),

    field_access: $ => seq($._expression, '.', $.identifier),

    index_access: $ => seq($._expression, '[', $._expression, ']'),

    binary_expression: $ => choice(
      ...[
        ['||', 'logical_or'],
        ['&&', 'logical_and'],
        ['|', 'bitwise_or'],
        ['^', 'bitwise_xor'],
        ['&', 'bitwise_and'],
        ['==', 'eq'], ['!=', 'ne'],
        ['<', 'lt'], ['>', 'gt'], ['<=', 'le'], ['>=', 'ge'],
        ['<<', 'shl'], ['>>', 'shr'],
        ['+', 'add'], ['-', 'sub'],
        ['*', 'mul'], ['/', 'div'], ['%', 'rem'],
      ].map(([op, name], index) => prec.left(10 + index, seq($._expression, op, $._expression)))
    ),

    unary_expression: $ => prec(3, seq(
      choice('-', '!', '~', '*', '&', '&&', 'mut'),
      $._expression
    )),

    cast_expression: $ => prec.right(2, seq($._expression, 'as', $.type_annotation)),

    as_expression: $ => prec.right(2, seq($._expression, 'as', $.type_annotation)),

    block_expression: $ => $.block,

    if_expression: $ => seq(
      'if',
      $._expression,
      $.block,
      optional(seq('else', choice($.block, $.if_expression)))
    ),

    match_expression: $ => seq(
      'match',
      $._expression,
      '{',
      commaSep($.match_arm),
      '}'
    ),

    lambda_expression: $ => seq(
      '|',
      commaSep($.lambda_parameter),
      '|',
      optional(seq('->', $.type_annotation)),
      choice($._expression, $.block)
    ),

    lambda_parameter: $ => seq(
      optional('mut'),
      $.identifier,
      optional(seq(':', $.type_annotation))
    ),

    // Quantum Intrinsics
    quantum_intrinsic: $ => choice(
      $.qalloc_call,
      $.qfree_call,
      $.hadamard_call,
      $.cnot_call,
      $.measure_call,
      $.bell_pair_call,
      $.qft_call,
      $.grover_oracle_call,
    ),

    qalloc_call: $ => seq(
      'qalloc',
      optional($.generic_arguments),
      '(',
      optional(commaSep($._expression)),
      ')'
    ),

    qfree_call: $ => seq('qfree', '(', $._expression, ')'),

    hadamard_call: $ => seq('hadamard', '(', $._expression, ')'),

    cnot_call: $ => seq('cnot', '(', $._expression, ',', $._expression, ')'),

    measure_call: $ => seq('measure', '(', $._expression, ')'),

    bell_pair_call: $ => seq('bell_pair', '(', ')'),

    qft_call: $ => seq('qft', '(', $._expression, ')'),

    grover_oracle_call: $ => seq(
      'grover_oracle',
      '(',
      $._expression,
      ',',
      $._expression,
      ')'
    ),

    // Tensor Operations
    tensor_operation: $ => choice(
      $.matmul_call,
      $.add_call,
      $.sub_call,
      $.scale_call,
      $.transpose_call,
      $.contract_call,
      $.outer_product_call,
      $.dot_call,
    ),

    matmul_call: $ => seq('matmul', '(', $._expression, ',', $._expression, ')'),

    add_call: $ => seq('add', '(', $._expression, ',', $._expression, ')'),

    sub_call: $ => seq('sub', '(', $._expression, ',', $._expression, ')'),

    scale_call: $ => seq('scale', '(', $._expression, ',', $._expression, ')'),

    transpose_call: $ => seq('transpose', '(', $._expression, ')'),

    contract_call: $ => seq(
      'contract',
      '(',
      $._expression,
      ',',
      $._expression,
      ',',
      commaSep($.identifier),
      ')'
    ),

    outer_product_call: $ => seq('outer_product', '(', $._expression, ',', $._expression, ')'),

    dot_call: $ => seq('dot', '(', $._expression, ',', $._expression, ')'),

    // Polyhedral expressions
    polyhedral_expression: $ => choice(
      $.forall_expression,
      $.tile_expression,
    ),

    forall_expression: $ => seq(
      'forall',
      optional($.schedule_annotation),
      '(',
      commaSep($.forall_binding),
      ')',
      $._expression
    ),

    tile_expression: $ => seq(
      'tile',
      $.schedule_annotation,
      '(',
      commaSep($.identifier),
      ')',
      $._expression
    ),

    // Literals
    literal: $ => choice(
      $.number_literal,
      $.string_literal,
      $.char_literal,
      $.bool_literal,
      $.unit_literal,
    ),

    number_literal: $ => choice(
      /[0-9]+(_[0-9]+)*([uif](8|16|32|64|128|size))?/,
      /0x[0-9a-fA-F]+(_[0-9a-fA-F]+)*/,
      /0o[0-7]+(_[0-7]+)*/,
      /0b[01]+(_[01]+)*/,
    ),

    string_literal: $ => seq('"', repeat(choice(/[^"\\]/, '\\.')), '"'),

    char_literal: $ => seq("'", choice(/[^'\\]/, '\\.'), "'"),

    bool_literal: $ => choice('true', 'false'),

    unit_literal: $ => seq('(', ')'),

    array_literal: $ => seq('[', commaSep($._expression), ']'),

    tuple_literal: $ => seq('(', commaSep1($._expression), ')'),

    struct_literal: $ => prec(1, seq(
      $.identifier,
      '{',
      commaSep($.field_initializer),
      '}'
    )),

    field_initializer: $ => seq($.identifier, ':', $._expression),

    generic_arguments: $ => seq('<', commaSep($.type_annotation), '>'),

    identifier: $ => /[a-zA-Z_][a-zA-Z0-9_]*/,

    comment: $ => token(choice(
      seq('//', /.*/),
      seq('/*', /[^*]*\*+([^/*][^*]*\*+)*/, '/'),
    )),
  }
});

function commaSep(rule) {
  return optional(seq(rule, repeat(seq(',', rule)), optional(',')));
}

function commaSep1(rule) {
  return seq(rule, repeat(seq(',', rule)), optional(','));
}