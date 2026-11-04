import { CoffeeCard, label } from './CoffeeCard'
import * as Cards from './CoffeeCard'
import { useState } from 'react'
import * as cards from './CoffeeCard'
import { $CoffeeCard as $Card, _CoffeeCard as _Card } from './CoffeeCard'
import { Basket, orderTotal } from './Basket'
import type { BasketItem } from './Basket'

export function App() {
  return (
    <main>
      <CoffeeCard name="espresso" title={label('espresso')} price={2.5} />
      <Cards.CoffeeCard name="latte" price={3.5}></Cards.CoffeeCard>
    </main>
  )
}

// A prop with the same name is not evidence for the imported component.
export function Preview({ CoffeeCard }: { CoffeeCard: typeof Cards.CoffeeCard }) {
  return <CoffeeCard name="preview" />
}

export function Menu({ basket }: { basket: Basket }) {
  const [items, setItems] = useState<BasketItem[]>(() => basket.snapshot())
  const addDrink = (function addToBasket(name: string, quantity = 1): void {
    basket.add(name, quantity)
    setItems(basket.snapshot())
  })
  return (
    <>
      <section aria-label="Coffee menu">
        <cards.CoffeeCard name="espresso" price={2.5} onOrder={addDrink} />
        <$Card name="filter" price={3} onOrder={(name) => addDrink(name)} />
        <_Card name="latte" price={3.5} onOrder={addDrink} />
      </section>
      <aside aria-label="Your order" aria-live="polite">
        <ul>{items.map(({ name, quantity }) => <li key={name}>{quantity} x {name}</li>)}</ul>
        <output aria-label="Total">EUR {orderTotal(basket).toFixed(2)}</output>
      </aside>
    </>
  )
}

export function CoffeeCounter() {
  const [basket] = useState<Basket>(() => new Basket())
  return <main><h1>Matis Coffee</h1><Menu basket={basket} /></main>
}
