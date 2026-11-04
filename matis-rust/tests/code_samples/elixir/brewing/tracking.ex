defmodule Coffee.Brewing.Tracking do
  alias Coffee.Brewing.Order

  def quote(%Order{names: names} = order, fee \\ 0) when names != [] do
    Order.estimate(order) + fee
  end

  def empty, do: []
  def labels do
    format = fn %Order{names: names} = order, counter when names != [] -> "#{Order.preview(order)} at #{counter}" end
    blank = fn -> empty() end
    {format, blank}
  end

  def dispatch(orders, counter) do
    {format, blank} = labels()
    Enum.reduce(orders, blank.(), fn order, tickets ->
      if order.names == [], do: tickets, else: tickets ++ [format.(order, counter)]
    end)
  end
end
