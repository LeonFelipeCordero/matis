defmodule Coffee.Brewing.Scheduling do
  use GenServer
  require Logger
  alias Coffee.Brewing.Recipe

  def start_link(names), do: GenServer.start_link(__MODULE__, names)

  @impl true
  def init(names), do: {:ok, names}

  def schedule(names, multiplier \\ 1), do: plan(names, multiplier)

  defp plan(names, multiplier \\ 1) when is_list(names) do
    names |> prepare(multiplier) |> dispatch
  end

  defp prepare(names, multiplier) do
    names |> Enum.map(&Recipe.new/1) |> Enum.map(&Recipe.scale(&1, multiplier))
  end

  defp dispatch(names) do
    Logger.info("scheduled #{length(names)} drinks")
    names
  end
end
