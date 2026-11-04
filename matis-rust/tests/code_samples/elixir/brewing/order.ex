defmodule Coffee.Brewing.Order do
  alias Coffee.Brewing.Recipe, as: Drink
  alias Coffee.Reporting.Report, as: Receipt

  defstruct customer: "walk-in", names: []

  def new(customer, names \\ []) when is_binary(customer) do
    normalize = &String.downcase(String.trim(&1))
    %__MODULE__{customer: customer, names: Enum.map(names, normalize)}
  end

  def estimate(%__MODULE__{names: names}, multiplier \\ 1) do
    names
    |> Enum.map(&Drink.new/1)
    |> Enum.map(&Drink.scale(&1, multiplier))
    |> Enum.map(&Drink.dose/1)
    |> Enum.sum()
  end

  def preview(%__MODULE__{customer: customer, names: names}) do
    "#{customer}: #{Receipt.render(names)}"
  end
end
