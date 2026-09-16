; Highlights for Naso language

; Keywords
(keyword) @keyword

; Types
(primitive_type) @type.builtin
(quantum_type) @type.builtin
(tensor_type) @type.builtin
(linear_type) @type.builtin
(reference_type) @type.builtin
(function_type) @type.builtin
(tuple_type) @type.builtin
(array_type) @type.builtin
(struct_type) @type.builtin
(generic_type) @type.builtin

; Type annotations
(type_annotation) @type

; Function definitions
(function_definition) @function
(quantum_function_definition) @function
(tensor_function_definition) @function
(polyhedral_function_definition) @function

(function_signature
  name: (identifier) @function.name)

; Parameters
(parameter) @variable.parameter
(linear_parameter
  name: (identifier) @variable.parameter)
(inout_parameter
  name: (identifier) @variable.parameter)
(regular_parameter
  name: (identifier) @variable.parameter)
(quantum_parameter
  name: (identifier) @variable.parameter)

; Quantum intrinsics
(quantum_intrinsic) @function.builtin
(quantum_intrinsic_call) @function.call

(quantum_intrinsic) @function.macro
#match?(@function.macro, "^(qalloc|qfree|hadamard|pauli_x|pauli_y|pauli_z|phase|cnot|cz|swap|toffoli|fredkin|measure|bell_pair|ghz_state|w_state|qft|grover_oracle|grover_diffusion|amplitude_amplification|phase_estimation|quantum_teleportation|superdense_coding|reset|barrier|delay)$")

; Tensor intrinsics
(tensor_intrinsic) @function.builtin
(tensor_operation) @function.call

(tensor_intrinsic) @function.macro
#match?(@function.macro, "^(matmul|matvec|vecmat|outer_product|inner_product|tensor_product|contract|transpose|permute|reshape|slice|concatenate|split|einsum|svd|qr|eig|cholesky|lu|inv|det|trace|norm|normalize|softmax|log_softmax)$")

; Polyhedral constructs
(forall_loop) @keyword.repeat
(forall_header) @keyword.repeat
(tile_loop) @keyword.repeat
(tile_header) @keyword.repeat
(fuse_loop) @keyword.repeat
(fuse_header) @keyword.repeat

(schedule_annotation) @attribute
(schedule_policy) @constant

; Module system
(module_declaration) @module
(module_declaration
  name: (identifier) @module.name)

(import_declaration) @keyword.import
(import_declaration
  (identifier) @module.imported)

(prelude_import) @keyword.import

; Variables and constants
(let_declaration
  name: (pattern) @variable.declaration)

(const_declaration
  name: (identifier) @constant.declaration)

(constant_declaration
  name: (identifier) @constant.declaration)

; Literals
(number_literal) @number
(float_literal) @number.float
(string_literal) @string
(bool_literal) @boolean
(qubit_literal) @constant.builtin
(tensor_literal) @constant.builtin

; Operators
(binary_expression) @operator
(unary_expression) @operator

; Control flow
(if_statement) @keyword.conditional
(while_statement) @keyword.repeat
(for_statement) @keyword.repeat
(return_statement) @keyword.return
(match_expression) @keyword.conditional

; Structs and enums
(struct_definition) @type
(struct_definition
  name: (identifier) @type.name)

(struct_field
  name: (identifier) @variable.member)

(field_attribute) @attribute

; Type aliases
(type_alias) @type
(type_alias
  name: (identifier) @type.name)

; Comments
(comment) @comment

; Doc comments
(comment) @comment.documentation
#match?(@comment.documentation, "^///")

; Attributes
(field_attribute) @attribute

; Macros
(quantum_intrinsic) @function.macro
(tensor_intrinsic) @function.macro

; Special identifiers
(identifier) @variable
#match?(@variable, "^(self|Self|super|crate)$") @variable.builtin

; Lifetime
(lifetime) @type.builtin

; Where clause
(where_clause) @keyword
(where_predicate) @keyword

; Type parameters
(type_parameter) @type.parameter
(type_constraint) @type.parameter

; Defer/unsafe
(defer_statement) @keyword
(unsafe_block) @keyword
