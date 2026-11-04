"""Coffee parsing dispatch includes local and package imports, not type guards."""

from typing import TYPE_CHECKING
from . import java
from .kotlin import extract_structure

if TYPE_CHECKING:
    from matis.domain.models import ParsedFile


def brew_go(root):
    from .go import extract_structure
    return extract_structure(root)


def brew_rust(root):
    from . import rust
    return rust.extract_structure(root)


def brew_java(root):
    return java.extract_structure(root)


def brew_kotlin(root):
    return extract_structure(root)
