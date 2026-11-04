export interface BasketItem {
  name: string
  quantity: number
}

export class Basket {
  private items: BasketItem[] = []

  constructor(private readonly prices: Record<string, number> = { espresso: 2.5, filter: 3, latte: 3.5 }) {}

  add(name: string, quantity = 1): void {
    const item = this.items.find(item => item.name === name)
    if (item) item.quantity += quantity
    else this.items.push({ name, quantity })
  }

  total(discount = 0, ...fees: number[]): number {
    const subtotal = this.items.reduce((sum, { name, quantity }) => sum + this.prices[name] * quantity, 0)
    return subtotal * (1 - discount) + fees.reduce((sum, fee) => sum + fee, 0)
  }

  snapshot(): BasketItem[] {
    return this.items.map(item => ({ ...item }))
  }
}

export function startOrder(name: string, quantity = 1): Basket {
  const basket: Basket = new Basket()
  basket.add(name, quantity)
  return basket
}

export function orderTotal(basket: Basket, discount = 0): number {
  return basket.total(discount)
}
