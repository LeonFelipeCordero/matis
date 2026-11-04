local function label(order)
    return order.name
end

local function dispatch(order)
    print(label(order))
end

dispatch({name = "coffee"})
