defmodule Coffee.Reporting.Report do
  alias Coffee.Brewing.{Inventory, Recipe}

  def total(names), do: Enum.reduce(names, 0, fn name, sum -> sum + Recipe.portions(name) end)

  def render(names) do
    entries = Enum.map_join(names, ",", fn name -> "#{name}=#{Recipe.portions(name)}" end)
    "coffee:#{entries};total=#{total(names)}"
  end

  def daily(names, stock) do
    available = Inventory.available(stock, names)
    counts = Enum.map(names, &Recipe.portions/1)
    "#{render(names)};available=#{length(available)};largest=#{largest(counts)}"
  end

  defp largest([]), do: 0
  defp largest([count | rest]) when count >= 0, do: max(count, largest(rest))
end
