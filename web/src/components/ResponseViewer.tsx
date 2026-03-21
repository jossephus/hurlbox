import { useEffect, useState, useMemo } from 'react'
import { codeToHtml } from 'shiki'

interface ResponseViewerProps {
  content: string
  theme: 'dark' | 'light'
}

export function ResponseViewer({ content, theme }: ResponseViewerProps) {
  const [html, setHtml] = useState<string>('')

  const parsed = useMemo(() => {
    if (!content) return null
    
    // Split content into header line and body
    const lines = content.split('\n')
    const header = lines[0]
    const body = lines.slice(1).join('\n').trim()
    
    // Try to parse body as JSON
    try {
      const jsonBody = JSON.parse(body)
      const formatted = JSON.stringify(jsonBody, null, 2)
      return { header, body: formatted, isJson: true }
    } catch {
      return { header, body, isJson: false }
    }
  }, [content])

  useEffect(() => {
    if (!parsed) {
      setHtml('')
      return
    }

    if (parsed.isJson) {
      codeToHtml(parsed.body, {
        lang: 'json',
        theme: theme === 'dark' ? 'github-dark-dimmed' : 'github-light',
      }).then(jsonHtml => {
        setHtml(jsonHtml)
      }).catch(() => {
        setHtml(`<pre style="margin:0;color:var(--text-primary)">${parsed.body}</pre>`)
      })
    } else {
      setHtml(`<pre style="margin:0;color:var(--text-primary)">${parsed.body}</pre>`)
    }
  }, [parsed, theme])

  if (!content) {
    return (
      <div style={{ color: 'var(--text-muted)', fontFamily: 'var(--font-mono)', fontSize: '12px' }}>
        Response will appear here...
      </div>
    )
  }

  if (!parsed) return null

  return (
    <div className="response-viewer" style={{ fontFamily: 'var(--font-mono)', fontSize: '12px' }}>
      {/* Status header */}
      <div 
        className="mb-3 pb-2"
        style={{ 
          color: parsed.header.includes('✓') ? 'var(--accent-green)' : 'var(--text-primary)',
          borderBottom: '1px solid var(--border-dim)',
        }}
      >
        {parsed.header}
      </div>
      
      {/* Body with syntax highlighting */}
      <div className="response-body" dangerouslySetInnerHTML={{ __html: html }} />
    </div>
  )
}
