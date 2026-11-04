defmodule Coffee.Reservations.Ledger do
  def commit(order), do: {:reserved, order}
end

defmodule Coffee.Reservations.Intake do
  alias Coffee.Reservations.Ledger

  def fulfill(order, ledger) do
    Ledger.commit(order)
    ledger.commit(order)
  end
end
