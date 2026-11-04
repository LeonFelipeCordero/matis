export interface CoffeeProps {
  name: string
  title?: string
  price?: number
  onOrder?: (name: string) => void
  soldOut?: boolean
}

export function label(name: string): string {
  return `Coffee: ${name}`
}

export function CoffeeCard({ name, title, price = 3, onOrder, soldOut = false }: CoffeeProps) {
  return (
    <article aria-label={name} title={title}>
      <h2>{label(name)}</h2>
      <p>EUR {price.toFixed(2)}</p>
      {onOrder && <button type="button" disabled={soldOut} onClick={() => onOrder(name)}>Add {name}</button>}
    </article>
  )
}

export { CoffeeCard as $CoffeeCard, CoffeeCard as _CoffeeCard }
