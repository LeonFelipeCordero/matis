import { useState } from 'react'
import { useMutation, useQueryClient } from '@tanstack/react-query'
import { Plus } from 'lucide-react'
import { api } from '@/lib/api'
import { Button } from '@/components/ui/button'
import { Input } from '@/components/ui/input'

export function RepositoryForm({ onCreated }: { onCreated: () => void }) {
  const client = useQueryClient()
  const [name, setName] = useState('')
  const [location, setLocation] = useState('')
  const mutation = useMutation({
    mutationFn: () =>
      api('/api/repositories', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ name, location }),
      }),
    onSuccess: async () => {
      await client.invalidateQueries({ queryKey: ['repositories'] })
      onCreated()
    },
  })
  return (
    <form
      className="register-form"
      onSubmit={(event) => {
        event.preventDefault()
        mutation.mutate()
      }}
    >
      <div className="form-intro">
        <h2>Give your repository a home</h2>
        <p>
          Register it here, then index its local sources from the terminal.
          Nothing runs automatically.
        </p>
      </div>
      <div className="form-fields">
        <label htmlFor="repository-name">
          Repository name
          <Input
            id="repository-name"
            name="name"
            placeholder="e.g. matis"
            required
            maxLength={200}
            value={name}
            onChange={(event) => setName(event.target.value)}
            autoFocus
          />
        </label>
        <label htmlFor="repository-location">
          URL or local path
          <Input
            id="repository-location"
            name="location"
            placeholder="https://github.com/team/project"
            required
            maxLength={4096}
            value={location}
            onChange={(event) => setLocation(event.target.value)}
            aria-describedby="location-help"
          />
        </label>
        <p id="location-help" className="field-help">
          For reference only. We don’t clone or access this location.
        </p>
        {mutation.error && (
          <p className="error-message" role="alert">
            {mutation.error.message}
          </p>
        )}
        <Button
          type="submit"
          disabled={mutation.isPending || !name.trim() || !location.trim()}
        >
          <Plus size={16} />
          {mutation.isPending ? 'Registering…' : 'Register repository'}
        </Button>
      </div>
    </form>
  )
}
