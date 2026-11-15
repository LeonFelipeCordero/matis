(function_signature
  name: (identifier) @name) @definition.function

(method_signature
  name: (property_identifier) @name) @definition.method

(abstract_method_signature
  name: (property_identifier) @name) @definition.method

(abstract_class_declaration
  name: (type_identifier) @name) @definition.class

(module
  name: (identifier) @name) @definition.module

(interface_declaration
  name: (type_identifier) @name) @definition.interface

(type_annotation
  (type_identifier) @name) @reference.type

(new_expression
  constructor: (identifier) @name) @reference.class

[(function_declaration) (generator_function_declaration)] @definition.function
[(class_declaration) (class)] @definition.class
(method_definition) @definition.method
(interface_body (property_signature name: (_) @name) @definition.method)
[(arrow_function) (function_expression) (generator_function)] @definition.function
