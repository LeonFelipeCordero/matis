(enum_body
  (enum_constant name: (identifier) @binding))

; shortcut: direct positive if conditions only; use flow analysis for compound conditions.
(if_statement
  condition: (parenthesized_expression
    (instanceof_expression name: (identifier) @binding)))
