; Locals for Naso language

; Module scope
(module_declaration) @local.scope
(module_declaration
  name: (identifier) @local.definition.module)

; Function scope
(function_definition) @local.scope
(function_definition
  name: (identifier) @local.definition.function)

(quantum_function_definition) @local.scope
(quantum_function_definition
  name: (identifier) @local.definition.function)

(tensor_function_definition) @local.scope
(tensor_function_definition
  name: (identifier) @local.definition.function)

(polyhedral_function_definition) @local.scope
(polyhedral_function_definition
  name: (identifier) @local.definition.function)

; Block scope
(block) @local.scope

; Parameters
(linear_parameter
  name: (identifier) @local.definition.parameter)

(inout_parameter
  name: (identifier) @local.definition.parameter)

(regular_parameter
  name: (identifier) @local.definition.parameter)

(quantum_parameter
  name: (identifier) @local.definition.parameter)

; Variables
(let_declaration
  name: (pattern) @local.definition.var)

(const_declaration
  name: (identifier) @local.definition.var)

(constant_declaration
  name: (identifier) @local.definition.constant)

; Pattern matching
(tuple_pattern
  (pattern) @local.definition.var)

(struct_pattern
  (pattern) @local.definition.var)

(field_pattern
  name: (identifier) @local.definition.var)

(quantum_pattern
  name: (identifier) @local.definition.var)

; Struct definitions
(struct_definition) @local.scope
(struct_definition
  name: (identifier) @local.definition.type)

(struct_field
  name: (identifier) @local.definition.field)

; Type aliases
(type_alias
  name: (identifier) @local.definition.type)

; Type parameters
(type_parameter
  name: (identifier) @local.definition.type_parameter)

; Loop variables
(forall_index
  name: (identifier) @local.definition.var)

(tile_dimension
  name: (identifier) @local.definition.var)

; Import scope
(import_declaration) @local.scope
(import_declaration
  (identifier) @local.definition.import)

(import_item
  name: (identifier) @local.definition.import
  alias: (identifier) @local.definition.import)

; References
(identifier) @local.reference
#not-kind? @local.reference "type_identifier"

; Function calls
(function_call
  function: (identifier) @local.reference.function)

(quantum_intrinsic_call
  function: (quantum_intrinsic) @local.reference.function)

(tensor_operation
  function: (tensor_intrinsic) @local.reference.function)

; Type references
(type_annotation
  (identifier) @local.reference.type)

(quantum_type
  (identifier) @local.reference.type)

(tensor_type
  (identifier) @local.reference.type)

(struct_type
  (identifier) @local.reference.type)

(generic_type
  (identifier) @local.reference.type)

; Module references
(module_path
  (identifier) @local.reference.module)

(import_declaration
  from: (module_path) @local.reference.module)

; Struct field access
(member_expression
  field: (identifier) @local.reference.field)

; Pattern references
(match_arm
  pattern: (pattern) @local.reference)

; Lambda parameters
(lambda_parameter
  name: (identifier) @local.definition.parameter)

; Capture references in closures
(lambda_expression) @local.scope
