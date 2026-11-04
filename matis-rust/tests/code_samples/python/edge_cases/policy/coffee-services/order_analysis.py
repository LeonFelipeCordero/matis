"""Analyze an in-memory coffee graph without acquiring persistence handles."""

from matis.domain.models import ParsedRepository


class OrderAnalysis:
    def __init__(self, repository: ParsedRepository):
        self.repository = repository

    def files(self):
        return self.repository.files

    def paths(self):
        return [file.source.path for file in self.files()]

    def languages(self):
        return sorted({file.source.language for file in self.files()})
