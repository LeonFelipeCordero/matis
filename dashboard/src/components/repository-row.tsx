import { Link } from '@tanstack/react-router'
import { ArrowRight, FolderGit2 } from 'lucide-react'
import { formatDate } from '@/lib/api'
import { Badge } from '@/components/ui/badge'
import type { Repository } from '@/lib/api'

export function RepositoryRow({ repository }: { repository: Repository }) {
  return (
    <Link
      to="/graph/$repositoryId"
      params={{ repositoryId: repository.id }}
      className="repository-row"
    >
      <span className="repository-icon">
        <FolderGit2 size={23} />
      </span>
      <div className="repository-info">
        <h3>{repository.name}</h3>
        <p title={repository.location}>{repository.location}</p>
      </div>
      <div className="repository-status">
        <Badge variant={repository.importedAt ? 'secondary' : 'outline'}>
          {repository.importedAt ? 'Ready to explore' : 'Awaiting index'}
        </Badge>
        <span>
          {repository.importedAt
            ? `Indexed ${formatDate(repository.importedAt)}`
            : 'Register → index → explore'}
        </span>
      </div>
      <ArrowRight size={19} className="row-arrow" />
    </Link>
  )
}
