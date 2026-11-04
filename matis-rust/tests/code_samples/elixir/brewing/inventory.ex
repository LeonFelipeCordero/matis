defmodule Coffee.Brewing.Inventory do
  import Coffee.Brewing.Recipe, only: [portions: 1]

  defstruct beans: %{}

  def new(beans \\ %{}), do: %__MODULE__{beans: beans}

  def remaining(%__MODULE__{beans: beans}, name), do: Map.get(beans, name, 0)

  def reserve(%__MODULE__{} = stock, name) do
    count = portions(name)
    cond do
      count == 0 -> {:error, :unknown}
      remaining(stock, name) < count -> {:error, :sold_out}
      true -> {:ok, %{stock | beans: Map.update!(stock.beans, name, &(&1 - count))}}
    end
  end

  def restock(stock, deliveries, adjust \\ &Function.identity/1) do
    Enum.reduce(deliveries, stock, fn {name, count}, current ->
      normalize = fn value -> max(adjust.(value), 0) end
      beans = Map.update(current.beans, name, normalize.(count), fn previous ->
        previous + normalize.(count)
      end)
      %{current | beans: beans}
    end)
  end

  def available(stock, names) do
    Enum.filter(names, fn
      name when is_binary(name) -> remaining(stock, name) >= portions(name) and portions(name) > 0
      _ -> false
    end)
  end
end
