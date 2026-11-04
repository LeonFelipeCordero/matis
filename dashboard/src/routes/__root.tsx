import { QueryClientProvider } from '@tanstack/react-query'
import {
  HeadContent,
  Link,
  Outlet,
  Scripts,
  createRootRouteWithContext,
} from '@tanstack/react-router'
import { Network } from 'lucide-react'
import appCss from '../styles.css?url'
import type { QueryClient } from '@tanstack/react-query'

export const Route = createRootRouteWithContext<{ queryClient: QueryClient }>()(
  {
    head: () => ({
      meta: [
        { charSet: 'utf-8' },
        { name: 'viewport', content: 'width=device-width, initial-scale=1' },
        { title: 'Matis · Repository graphs' },
        {
          name: 'description',
          content:
            'Explore your code, from folders to function calls. Powered by your Tree-sitter index.',
        },
      ],
      links: [
        { rel: 'stylesheet', href: appCss },
        { rel: 'icon', type: 'image/svg+xml', href: '/favicon.svg' },
        { rel: 'manifest', href: '/manifest.json' },
      ],
    }),
    component: RootComponent,
    shellComponent: RootDocument,
    notFoundComponent: () => (
      <main className="page empty-state">
        <h1>Page not found</h1>
        <Link to="/">Back to repositories</Link>
      </main>
    ),
    errorComponent: ({ error, reset }) => (
      <main className="page empty-state">
        <h1>Unable to open this page</h1>
        <p role="alert">
          {error instanceof Error ? error.message : 'Unexpected error'}
        </p>
        <button onClick={reset}>Try again</button>
      </main>
    ),
  },
)

function RootDocument({ children }: { children: React.ReactNode }) {
  return (
    <html lang="en">
      <head>
        <HeadContent />
      </head>
      <body>
        {children}
        <Scripts />
      </body>
    </html>
  )
}

function RootComponent() {
  const { queryClient } = Route.useRouteContext()
  return (
    <QueryClientProvider client={queryClient}>
      <a className="skip-link" href="#main">
        Skip to content
      </a>
      <header className="app-header">
        <Link to="/" className="brand" aria-label="Matis">
          <span className="brand-mark">
            <Network size={21} />
          </span>
          Matis
        </Link>
        <Link to="/" className="header-link">
          Repositories
        </Link>
      </header>
      <Outlet />
    </QueryClientProvider>
  )
}
