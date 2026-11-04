defmodule Coffee.Brewing.Recipe do
  alias Coffee.Reporting.Report

  defstruct name: "filter", cups: 1

  def new(name, cups \\ 1) when is_binary(name) and cups > 0 do
    %__MODULE__{name: String.downcase(String.trim(name)), cups: cups}
  end

  def portions(name) do
    case name do
      "espresso" -> 2
      "filter" -> 3
      _ -> 0
    end
  end

  def scale(%__MODULE__{cups: cups} = recipe, multiplier) when multiplier > 0 do
    %{recipe | cups: cups * multiplier}
  end

  def dose(%__MODULE__{name: name, cups: cups}), do: portions(name) * cups

  def preview(name), do: Report.render([name])
end
