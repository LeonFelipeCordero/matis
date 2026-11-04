"""Compatibility exports are not a second coffee composition root."""

from matis.services.factories import repository_service, navigation_service, migrate_database

__all__ = ["repository_service", "navigation_service", "migrate_database"]


def open_coffee_project(path):
    return repository_service(path)


def initialize_coffee_project(path):
    migrate_database(path)
