; Declaration-head ancestry is filtered in Rust so guard calls remain visible.
(call) @site
((call target: (identifier) @target) @exclude
  (#any-of? @target
    "defmodule" "defprotocol" "defimpl" "alias" "import" "use" "require"
    "if" "unless" "case" "cond" "with" "for" "receive" "try"
    "def" "defp" "defmacro" "defmacrop" "defguard" "defguardp" "defdelegate"))

; Capture only the RHS identifier, not the complete pipeline.
(binary_operator operator: "|>" right: (identifier) @site)
