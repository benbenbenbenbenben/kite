(function_declaration
  name: (identifier) @name)

(generator_function_declaration
  name: (identifier) @name)

(function_signature
  name: (identifier) @name)

(class_declaration
  name: (type_identifier) @name)

(abstract_class_declaration
  name: (type_identifier) @name)

(interface_declaration
  name: (type_identifier) @name)

(type_alias_declaration
  name: (type_identifier) @name)

(enum_declaration
  name: (identifier) @name)

(module
  name: (identifier) @name)

(internal_module
  name: (identifier) @name)

(method_definition
  name: (property_identifier) @name)

(method_definition
  name: (private_property_identifier) @name)

(method_signature
  name: (property_identifier) @name)

(method_signature
  name: (private_property_identifier) @name)

(abstract_method_signature
  name: (property_identifier) @name)

(abstract_method_signature
  name: (private_property_identifier) @name)

(public_field_definition
  name: (property_identifier) @name)

(public_field_definition
  name: (private_property_identifier) @name)

(lexical_declaration
  (variable_declarator
    name: (identifier) @name))

(variable_declaration
  (variable_declarator
    name: (identifier) @name))
