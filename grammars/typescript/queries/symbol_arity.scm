(function_declaration
  name: (identifier) @name
  parameters: (formal_parameters) @params)

(method_definition
  name: [(property_identifier) (private_property_identifier)] @name
  parameters: (formal_parameters) @params)

(method_signature
  name: [(property_identifier) (private_property_identifier)] @name
  parameters: (formal_parameters) @params)

(abstract_method_signature
  name: [(property_identifier) (private_property_identifier)] @name
  parameters: (formal_parameters) @params)

(generator_function_declaration
  name: (identifier) @name
  parameters: (formal_parameters) @params)

(function_signature
  name: (identifier) @name
  parameters: (formal_parameters) @params)
