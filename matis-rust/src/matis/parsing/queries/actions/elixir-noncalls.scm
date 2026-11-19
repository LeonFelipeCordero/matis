; Typespecs contain declaration and type syntax, not calls.
(unary_operator
  operator: "@"
  operand: (call target: (identifier) @attribute)
  (#any-of? @attribute "spec" "callback" "macrocallback" "type" "typep" "opaque")) @noncall

; A remote name with no argument list in a name/arity capture is a reference.
(unary_operator
  operator: "&"
  operand: (binary_operator
    left: (call target: (dot) .) @noncall
    operator: "/"
    right: (integer)))
