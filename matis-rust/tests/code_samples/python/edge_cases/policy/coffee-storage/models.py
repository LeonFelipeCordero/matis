"""Coffee order declarations contain schema, never database or filesystem I/O."""

from sqlalchemy import Integer, String
from sqlalchemy.orm import DeclarativeBase, Mapped, mapped_column


class Base(DeclarativeBase):
    pass


class CoffeeOrder(Base):
    __tablename__ = "coffee_orders"
    id: Mapped[int] = mapped_column(Integer, primary_key=True)
    customer: Mapped[str] = mapped_column(String(100))

    def label(self):
        return f"Coffee order for {self.customer}"


class ActiveOrder(CoffeeOrder):
    def execute(self, statement):
        return statement

    def save_order(self, session):
        session.add(self)
        session.commit()

    def load_order(self, session):
        return session.query(CoffeeOrder).first()

    def export_order(self, destination):
        destination.write_text(self.customer)

    def print_order(self):
        print(self.customer)

    def open_export(self):
        return open("coffee-orders.csv")
