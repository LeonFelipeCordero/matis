defmodule Coffee.Brewing.Tasting do
  def brew([]), do: "empty"
  def brew(names), do: names
  def retry(n) when n <= 0, do: 0
  def retry(n), do: retry(n - 1)
  def serve do
    IO.puts(brew([]))
    [] |> brew |> IO.puts()
    if true, do: brew(["espresso"])
    case 1 do
      1 -> brew(["filter"])
      _ -> "none"
    end
  end
end
