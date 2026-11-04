defmodule Coffee.Brewing.Brewer do
  alias Coffee.Brewing.Inventory
  alias Coffee.Brewing.Order
  alias Coffee.Brewing.Recipe
  alias Coffee.Reporting.Report
  require Logger

  def brew([]), do: Report.render([])

  def brew(names) do
    names |> Enum.filter(fn name -> Recipe.portions(name) > 0 end) |> Report.render()
  end

  def serve(stock, %Order{names: names}) do
    {accepted, remaining} = Enum.reduce(names, {[], stock}, fn name, {drinks, current} ->
      case Inventory.reserve(current, name) do
        {:ok, next} -> {[name | drinks], next}
        {:error, reason} ->
          unless reason == :unknown, do: Logger.info("sold out: #{name}")
          {drinks, current}
      end
    end)
    {brew(Enum.reverse(accepted)), remaining}
  end

  def retry(attempts) when attempts <= 0, do: 0
  def retry(attempts), do: 1 + retry(attempts - 1)

  def score(batches) do
    Enum.reduce(batches, 0, fn batch, sum ->
      Enum.reduce(batch, sum, fn name, subtotal ->
        count = Recipe.portions(name)
        if count > 0 and count <= 3 do
          subtotal + count
        else
          subtotal
        end
      end)
    end)
  end

  def classify(amount) do
    cond do
      amount <= 0 -> :empty
      amount <= 3 -> :counter
      amount <= 8 -> :table
      true -> :catering
    end
  end

  def ready(water, beans, cups, temperature) do
    water > 0 and beans > 0 and cups > 0 and temperature >= 90 and temperature <= 96
  end

  # Batch water in milliliters, corrected for temperature and grind size.
  def calibrate(beans, ratio, water_offset, batches, temperature, target_temperature, grind, target_grind) do
    (beans * ratio + water_offset) * batches + (temperature - target_temperature) * 2 + (grind - target_grind) * 5
  end

  def main do
    stock = Inventory.new(%{"espresso" => 12, "filter" => 9})
    stock = Inventory.restock(stock, [{"filter", 3}], &max(&1, 0))
    order = Order.new("Maya", ["espresso", "filter", "unknown"])
    {receipt, remaining} = serve(stock, order)
    IO.puts(receipt)
    IO.puts(brew([]))
    IO.puts(Recipe.preview("espresso"))
    IO.puts(Report.daily(order.names, remaining))
    IO.puts(Order.preview(order))
    IO.inspect(Order.estimate(order))
    IO.inspect({retry(2), score([order.names]), classify(length(order.names))})
    IO.inspect({ready(600, 40, 3, 93), calibrate(40, 15, 0, 1, 93, 93, 6, 6)})
  end
end
