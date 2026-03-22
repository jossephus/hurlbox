import { useState, useEffect } from 'react'
import { ChevronRight, ChevronDown, File, Folder, Plus } from 'lucide-react'

interface FileNode {
  name: string
  path: string           // Full absolute path (for API)
  relative_path: string  // Relative path for display
  type: 'file' | 'folder'
  children?: FileNode[]
}

interface FileExplorerProps {
  rootPath: string
  refreshKey: number
  onCreateFile: (directoryPath: string) => void
  onFileSelect: (path: string, content: string) => void
  selectedPath: string | null
}

function FileTreeNode({ 
  node, 
  depth, 
  onCreateFile,
  onFileSelect, 
  selectedPath 
}: { 
  node: FileNode
  depth: number
  onCreateFile: (directoryPath: string) => void
  onFileSelect: (path: string, content: string) => void
  selectedPath: string | null
}) {
  const [isExpanded, setIsExpanded] = useState(depth < 2)
  const [isLoading, setIsLoading] = useState(false)
  const isFolder = node.type === 'folder'
  const isSelected = node.relative_path === selectedPath

  const handleClick = async () => {
    if (isFolder) {
      setIsExpanded(!isExpanded)
    } else {
      // Load file content
      setIsLoading(true)
      try {
        const res = await fetch(`/api/file?path=${encodeURIComponent(node.relative_path)}`)
        if (res.ok) {
          const data = await res.json()
          onFileSelect(node.relative_path, data.content)
        }
      } catch (error) {
        console.error('Failed to load file:', error)
      } finally {
        setIsLoading(false)
      }
    }
  }

  return (
    <div>
      <div
        className="flex items-center gap-1 py-1 px-2 rounded transition-colors"
        style={{ 
          paddingLeft: `${depth * 12 + 8}px`,
          background: isSelected ? 'var(--bg-elevated)' : 'transparent',
          color: isSelected ? 'var(--text-primary)' : 'var(--text-secondary)',
        }}
      >
        <button
          type="button"
          className="flex min-w-0 flex-1 items-center gap-1 text-left cursor-pointer"
          onClick={handleClick}
        >
          {isFolder ? (
            <>
              {isExpanded ? (
                <ChevronDown className="w-3.5 h-3.5" style={{ color: 'var(--text-muted)' }} />
              ) : (
                <ChevronRight className="w-3.5 h-3.5" style={{ color: 'var(--text-muted)' }} />
              )}
              <Folder className="w-3.5 h-3.5" style={{ color: 'var(--accent-cyan)' }} />
            </>
          ) : (
            <>
              <span className="w-3.5" />
              <File className="w-3.5 h-3.5" style={{ color: 'var(--text-muted)' }} />
            </>
          )}
          <span className="text-xs truncate" style={{ fontFamily: 'var(--font-mono)' }}>
            {isLoading ? '...' : node.name}
          </span>
        </button>
        {isFolder && (
          <button
            type="button"
            className="p-1 rounded transition-colors hover:bg-[var(--bg-elevated)]"
            title={`New file in ${node.name}`}
            onClick={(event) => {
              event.stopPropagation()
              onCreateFile(node.relative_path)
            }}
          >
            <Plus className="w-3.5 h-3.5" style={{ color: 'var(--text-muted)' }} />
          </button>
        )}
      </div>
      {isFolder && isExpanded && node.children && (
        <div>
          {node.children.map((child) => (
            <FileTreeNode
              key={child.path}
              node={child}
              depth={depth + 1}
              onCreateFile={onCreateFile}
              onFileSelect={onFileSelect}
              selectedPath={selectedPath}
            />
          ))}
        </div>
      )}
    </div>
  )
}

export function FileExplorer({ rootPath, refreshKey, onCreateFile, onFileSelect, selectedPath }: FileExplorerProps) {
  const [fileTree, setFileTree] = useState<FileNode | null>(null)
  const [isLoading, setIsLoading] = useState(true)
  const [error, setError] = useState<string | null>(null)

  useEffect(() => {
    void refreshKey
    const fetchFiles = async () => {
      setIsLoading(true)
      setError(null)
      try {
        const res = await fetch(`/api/files?path=${encodeURIComponent(rootPath)}`)
        if (res.ok) {
          const data = await res.json()
          setFileTree(data)
        } else {
          const err = await res.json()
          setError(err.error || 'Failed to load files')
        }
      } catch (err) {
        setError('Failed to connect to server')
      } finally {
        setIsLoading(false)
      }
    }
    fetchFiles()
  }, [rootPath, refreshKey])

  if (isLoading) {
    return (
      <div className="p-3 text-xs" style={{ color: 'var(--text-muted)' }}>
        Loading files...
      </div>
    )
  }

  if (error) {
    return (
      <div className="p-3 text-xs" style={{ color: 'var(--accent-red, #ff5c5c)' }}>
        {error}
      </div>
    )
  }

  if (!fileTree || !fileTree.children || fileTree.children.length === 0) {
    return (
      <div className="p-3 text-xs" style={{ color: 'var(--text-muted)' }}>
        No .hurl files found
      </div>
    )
  }

  return (
    <div className="py-1">
      {fileTree.children.map((node) => (
          <FileTreeNode
            key={`${node.relative_path}/${node.name}`}
            node={node}
            depth={0}
            onCreateFile={onCreateFile}
            onFileSelect={onFileSelect}
            selectedPath={selectedPath}
          />
      ))}
    </div>
  )
}
