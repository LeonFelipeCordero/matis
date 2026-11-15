[(call_expression) (new_expression)] @site

; JSX member tags always count; bare tags exclude empty/native/custom-element names.
(jsx_opening_element name: (member_expression) @site)
(jsx_self_closing_element name: (member_expression) @site)
(jsx_opening_element name: (identifier) @site
  (#match? @site ".")
  (#not-match? @site "^[a-z]|-"))
(jsx_self_closing_element name: (identifier) @site
  (#match? @site ".")
  (#not-match? @site "^[a-z]|-"))
