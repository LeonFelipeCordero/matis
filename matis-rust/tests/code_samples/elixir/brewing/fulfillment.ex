defmodule Coffee.Brewing.Fulfillment do
  alias Coffee.Brewing.Recipe

  defmodule Client do
    @moduledoc "Legacy counter status callbacks for payment conditions and pickup cases."
    def if(true), do: :paid
    def if(false), do: :unpaid
    def case(true), do: :ready
    def case(false), do: :queued
  end

  def dispatch(paid, ready, stopped, paused) do
    portions = if paid && ready do
      unless stopped || paused, do: Recipe.portions("espresso"), else: 0
    else
      0
    end
    # Qualified client calls named after controls are not branching constructs.
    %{portions: portions, payment: Client.if(paid), pickup: Client.case(ready)}
  end

  def acknowledge(ready) do
    case ready do
      true -> :collect
      false -> :wait
      _ -> :unknown
    end
  end

  def priority(amount, ready, enabled) do
    cond do
      amount > 0 and ready -> if enabled, do: :collect
      amount < 0 -> :cancel
      true -> :wait
    end
  end

  def release(event, enabled, stopped, ready, valid, active) do
    if enabled do
      unless stopped do
        case event do
          :pickup when ready and valid -> if active, do: Recipe.portions("espresso")
          _ -> 0
        end
      end
    end
  end

  def accept_event(event, ready) do
    case event do
      _ when ready -> :collect
      _ -> :wait
    end
  end

  # Select generated APIs at expansion time; order state is checked at runtime.
  defmacro install_public(service) do
    if service == :counter do
      quote do
        def pickup(order), do: if(order.paid and order.ready, do: :collect, else: :wait)

        defmodule Receipt do
          def label(status), do: if(status == :collect, do: "Ready", else: "Pending")
        end
      end
    else
      quote do
        def pickup(order), do: {:delivery, order.address}
      end
    end
  end

  defmacro install_private(payment) do
    if payment == :prepaid do
      quote do
        def release(order), do: authorize_pickup(order)
        defp authorize_pickup(order), do: if(order.paid and order.ready, do: :collect, else: :hold)
      end
    else
      quote do
        def release(order), do: {:charge, order.total}
      end
    end
  end

  defmacro install_macro(schedule) do
    if schedule == :scheduled do
      quote do
        defmacro pickup_due(order) do
          quote do
            if unquote(order).ready and unquote(order).minutes_waiting >= 5,
              do: :notify,
              else: :wait
          end
        end
      end
    else
      quote do
        defmacro pickup_due(order) do
          quote do
            {:walk_in, unquote(order).ticket}
          end
        end
      end
    end
  end
end
