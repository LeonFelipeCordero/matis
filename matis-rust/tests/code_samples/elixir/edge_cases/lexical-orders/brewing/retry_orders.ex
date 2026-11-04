defmodule Coffee.Brewing.RetryOrders do
  alias Coffee.Reporting.Report

  def retry(names, attempts) when attempts <= 0, do: Report.render(names)
  def retry(names, attempts), do: retry(names, attempts - 1)
  def retry(names), do: retry(names, length(names))
end

defmodule Coffee.Inventory.RetryOrders do
  alias Coffee.Brewing.Inventory

  def retry(stock, names), do: retry(stock, names, length(names))
  def retry(stock, names, attempts) when attempts <= 0, do: {stock, names}
  def retry(stock, names, attempts) do
    updated = Inventory.restock(stock, Enum.map(names, &{&1, 1}), &max(&1, 0))
    retry(updated, names, attempts - 1)
  end
end
