import { useState, useRef, useCallback, useEffect } from 'react'
import Editor, { type OnMount } from '@monaco-editor/react'
import type * as monaco from 'monaco-editor'
import { TestTube2, FileText, Terminal, FileCode, Sun, Moon, Loader2 } from 'lucide-react'
import { ResponseViewer } from './components/ResponseViewer'
import { registerHurlLanguage } from './lib/hurl-lang'
import './App.css'

interface ExecutionResult {
  entry_index: number
  status: number
  headers: Record<string, string>
  body: string
  timing?: { duration_ms: number }
  assertions: Array<{ query: string; passed: boolean }>
  error?: string
}



interface EntryInfo {
  index: number
  start_line: number
  end_line: number
  method: string
  url: string
}

interface TestFileResponse {
  overall_pass: boolean
  total_assertions: number
  passed_assertions: number
  failed_assertions: number
  results: ExecutionResult[]
}

type RunMode = 'entry' | 'file' | 'test'

const SAMPLE_HURL = `GET https://jsonplaceholder.typicode.com/todos/
HTTP 200
[Asserts]
header "Content-Type" contains "application/json"

GET https://jsonplaceholder.typicode.com/todos/1
HTTP 200
[Asserts]
header "Content-Type" contains "application/json"`

function formatEntryResult(result: ExecutionResult, index: number): string {
  const statusIcon = result.status >= 200 && result.status < 300 ? '✓' : '✗'
  const lines = [
    `${statusIcon} Entry ${index + 1} — ${result.status}  ${result.timing?.duration_ms ? `(${result.timing.duration_ms}ms)` : ''}`,
  ]
  if (result.error) lines.push(`Error: ${result.error}`)
  lines.push('') // blank line before body
  lines.push(result.body || '')
  return lines.join('\n')
}

function formatResponseData(data: unknown, mode: RunMode): string {
  if (mode === 'test') {
    const t = data as TestFileResponse
    const icon = t.overall_pass ? '✓' : '✗'
    return `${icon} Tests: ${t.passed_assertions}/${t.total_assertions} passed\n${'─'.repeat(40)}\n${t.results.map((r, i) => formatEntryResult(r, i)).join('\n\n')}`
  }
  if (Array.isArray(data)) return data.map((r, i) => formatEntryResult(r, i)).join('\n\n')
  return formatEntryResult(data as ExecutionResult, 0)
}

function App() {
  const [content, setContent] = useState(SAMPLE_HURL)
  const [response, setResponse] = useState<string>('')
  const [isLoading, setIsLoading] = useState(false)
  const [entries, setEntries] = useState<EntryInfo[]>([])
  const [theme, setTheme] = useState<'dark' | 'light'>('dark')
  const [currentEntry, setCurrentEntry] = useState(0)
  const monacoRef = useRef<typeof monaco | null>(null)
  const editorRef = useRef<monaco.editor.IStandaloneCodeEditor | null>(null)
  const entriesRef = useRef<EntryInfo[]>([])
  const runRequestRef = useRef<((mode: RunMode, entryIndex?: number) => Promise<void>) | null>(null)

  useEffect(() => { entriesRef.current = entries }, [entries])

  const toggleTheme = useCallback(() => {
    setTheme(prev => {
      const newTheme = prev === 'dark' ? 'light' : 'dark'
      document.documentElement.classList.toggle('light', newTheme === 'light')
      return newTheme
    })
  }, [])

  const runRequest = useCallback(async (mode: RunMode, entryIndex?: number) => {
    setIsLoading(true)
    try {
      let endpoint = '/api/run-file'
      let body: Record<string, unknown> = { content, env: {} }

      if (mode === 'entry' && entryIndex !== undefined) {
        endpoint = '/api/run-entry'
        body = { content, entry_index: entryIndex, env: {} }
      } else if (mode === 'test') {
        endpoint = '/api/test-file'
      }

      const res = await fetch(endpoint, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify(body)
      })

      const data = await res.json()
      setResponse(formatResponseData(data, mode))
    } catch (error) {
      setResponse(`✗ Error: ${error}`)
    } finally {
      setIsLoading(false)
    }
  }, [content])

  useEffect(() => { runRequestRef.current = runRequest }, [runRequest])

  const parseEntries = useCallback(async (text: string) => {
    try {
      const res = await fetch('/api/parse', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ content: text })
      })
      if (res.ok) {
        const data = await res.json()
        setEntries(data.entries || [])
      }
    } catch {
      // silently fail
    }
  }, [])

  useEffect(() => {
    parseEntries(content)
  }, [content, parseEntries])

  useEffect(() => {
    if (editorRef.current && entries.length > 0) {
      setTimeout(() => {
        editorRef.current?.trigger('source', 'editor.action.codeLensRefresh', null)
      }, 50)
    }
  }, [entries])

  const handleEditorMount: OnMount = (editor, monacoInstance) => {
    monacoRef.current = monacoInstance
    editorRef.current = editor

    // Register Hurl language
    registerHurlLanguage(monacoInstance)

    monacoInstance.editor.registerCommand('aranshi.runEntry', (_accessor, ...args) => {
      const entryIndex = args[0] as number
      setCurrentEntry(entryIndex)
      runRequestRef.current?.('entry', entryIndex)
    })

    monacoInstance.languages.registerCodeLensProvider('hurl', {
      provideCodeLenses: () => {
        const lenses: monaco.languages.CodeLens[] = []
        const currentEntries = entriesRef.current

        for (const entry of currentEntries) {
          lenses.push({
            range: {
              startLineNumber: entry.start_line,
              startColumn: 1,
              endLineNumber: entry.start_line,
              endColumn: 1,
            },
            command: {
              id: 'aranshi.runEntry',
              title: '▶ Run',
              arguments: [entry.index],
            },
          })
        }

        return { lenses, dispose: () => { } }
      },
      resolveCodeLens: (_model, lens) => lens,
    })

    editor.addCommand(monacoInstance.KeyMod.CtrlCmd | monacoInstance.KeyCode.Enter, () => {
      runRequestRef.current?.('entry', currentEntry)
    })
  }

  return (
    <div 
      className="h-screen flex flex-col overflow-hidden"
      style={{ background: 'var(--bg-primary)' }}
    >
      <div className="noise-overlay" />

      <header 
        className="shrink-0 px-4 h-12 flex items-center justify-between"
        style={{ 
          background: 'var(--bg-secondary)',
          borderBottom: '1px solid var(--border-default)',
        }}
      >
        <div className="flex items-center gap-3">
          <span 
            className="text-sm font-semibold tracking-wide"
            style={{ 
              fontFamily: 'var(--font-mono)',
              color: 'var(--text-primary)',
            }}
          >
            HURLBOX
          </span>
          <span 
            className="text-xs"
            style={{ color: 'var(--text-muted)' }}
          >
            {entries.length > 0 ? `${entries.length} entries` : '—'}
          </span>
        </div>

        <div className="flex items-center gap-2">
          <button
            type="button"
            onClick={toggleTheme}
            className="p-1.5 rounded transition-colors"
            style={{ color: 'var(--text-muted)' }}
          >
            {theme === 'dark' ? <Moon className="w-4 h-4" /> : <Sun className="w-4 h-4" />}
          </button>

          <div className="w-px h-4" style={{ background: 'var(--border-default)' }} />

          <button
            type="button"
            onClick={() => runRequest('file')}
            disabled={isLoading}
            className="px-3 py-1.5 text-xs rounded transition-colors disabled:opacity-50"
            style={{ 
              color: 'var(--text-secondary)',
              background: 'transparent',
              border: '1px solid var(--border-default)',
            }}
          >
            <span className="flex items-center gap-1.5">
              <FileCode className="w-3.5 h-3.5" />
              Run File
            </span>
          </button>

          <button
            type="button"
            onClick={() => runRequest('test')}
            disabled={isLoading}
            className="px-3 py-1.5 text-xs rounded transition-colors disabled:opacity-50"
            style={{ 
              color: 'var(--text-primary)',
              background: 'var(--bg-elevated)',
              border: '1px solid var(--border-default)',
            }}
          >
            <span className="flex items-center gap-1.5">
              {isLoading ? (
                <Loader2 className="w-3.5 h-3.5 animate-spin" />
              ) : (
                <TestTube2 className="w-3.5 h-3.5" />
              )}
              Test
            </span>
          </button>
        </div>
      </header>

      <main className="flex-1 flex min-h-0">
        <div className="flex-1 flex flex-col min-w-0">
          <div 
            className="h-8 px-3 flex items-center shrink-0"
            style={{ 
              background: 'var(--bg-primary)',
              borderBottom: '1px solid var(--border-dim)',
            }}
          >
            <FileText className="w-3.5 h-3.5 mr-2" style={{ color: 'var(--text-muted)' }} />
            <span 
              className="text-xs"
              style={{ 
                fontFamily: 'var(--font-mono)',
                color: 'var(--text-secondary)',
              }}
            >
              editor.hurl
            </span>
          </div>
          <div className="flex-1 min-h-0">
            <Editor
              height="100%"
              defaultLanguage="hurl"
              theme="vs-dark"
              value={content}
              onChange={(v) => setContent(v || '')}
              onMount={handleEditorMount}
              options={{
                minimap: { enabled: false },
                fontSize: 13,
                fontFamily: 'JetBrains Mono, monospace',
                fontLigatures: true,
                lineNumbers: 'on',
                scrollBeyondLastLine: false,
                wordWrap: 'on',
                automaticLayout: true,
                codeLens: true,
                renderLineHighlight: 'line',
                cursorBlinking: 'smooth',
                cursorSmoothCaretAnimation: 'on',
                smoothScrolling: true,
                padding: { top: 12, bottom: 12 },
              }}
            />
          </div>
        </div>

        <div 
          className="w-[380px] flex flex-col min-w-0"
          style={{ 
            borderLeft: '1px solid var(--border-default)',
            background: 'var(--bg-secondary)',
          }}
        >
          <div 
            className="h-8 px-3 flex items-center justify-between shrink-0"
            style={{ 
              background: 'var(--bg-primary)',
              borderBottom: '1px solid var(--border-dim)',
            }}
          >
            <div className="flex items-center gap-2">
              <Terminal className="w-3.5 h-3.5" style={{ color: 'var(--text-muted)' }} />
              <span 
                className="text-xs"
                style={{ 
                  fontFamily: 'var(--font-mono)',
                  color: 'var(--text-secondary)',
                }}
              >
                response
              </span>
            </div>
            {isLoading && (
              <span className="text-[10px]" style={{ color: 'var(--text-muted)' }}>
                running...
              </span>
            )}
          </div>
          <div className="flex-1 overflow-y-auto p-3">
            <ResponseViewer content={response} />
          </div>
        </div>
      </main>

      <footer 
        className="shrink-0 h-6 px-3 flex items-center justify-between text-[10px]"
        style={{ 
          background: 'var(--bg-secondary)',
          borderTop: '1px solid var(--border-default)',
          color: 'var(--text-muted)',
          fontFamily: 'var(--font-mono)',
        }}
      >
        <span>{isLoading ? 'running...' : 'ready'}</span>
        <span>hurlbox</span>
      </footer>
    </div>
  )
}

export default App