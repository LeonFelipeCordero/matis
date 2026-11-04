defmodule Coffee.Reporting.Dispatch do
  alias Coffee.Reporting.Report
  alias Coffee.Brewing.Recipe

  @type formatter :: (String.t() -> String.t())

  def receipts(names, minimum) do
    classify = fn
      name when is_binary(name) -> if Recipe.portions(name) >= minimum, do: name, else: "pending"
      _ -> "invalid"
    end
    labels = Enum.map(names, fn name -> name end)
    originals = Enum.map(names, fn name -> name end)
    ready = Enum.map(labels, fn name ->
      Enum.map([name], fn item -> classify.(item) end)
    end)
    {originals, ready, Report.render(names)}
  end

  def render(names, formatter) do
    names |> Enum.map(formatter) |> Enum.join(",")
  end
end
