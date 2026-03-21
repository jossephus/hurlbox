import { useState, useRef, useCallback, useEffect } from 'react'
import Editor, { type OnMount } from '@monaco-editor/react'
import type * as monaco from 'monaco-editor'
import { TestTube2, FileText, FileCode, Sun, Moon, Loader2, FolderOpen, Folder } from 'lucide-react'
import { ResponseViewer } from './components/ResponseViewer'
import { FileExplorer } from './components/FileExplorer'
import { registerHurlLanguage } from './lib/hurl-lang'
import './App.css'

interface ExecutionResult {
  entry_index: number
  status: number
  headers: Record<string, string>
  body: string
  timing?: {
    duration_ms: number
    connect_time_ms?: number
    tls_time_ms?: number
    transfer_time_ms?: number
  }
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
  const [responseHeaders, setResponseHeaders] = useState<Record<string, string>>({})
  const [responseAssertions, setResponseAssertions] = useState<Array<{ query: string; passed: boolean }>>([])
  const [responseTiming, setResponseTiming] = useState<{ 
    duration_ms: number
    connect_time_ms?: number
    tls_time_ms?: number
    transfer_time_ms?: number
  } | null>(null)
  const [activeTab, setActiveTab] = useState<'response' | 'headers' | 'asserts' | 'request' | 'timing'>('response')
  const [lastRequest, setLastRequest] = useState<{ method: string; url: string } | null>(null)
  const [isLoading, setIsLoading] = useState(false)
  const [entries, setEntries] = useState<EntryInfo[]>([])
  const [theme, setTheme] = useState<'dark' | 'light'>('dark')
  const [currentEntry, setCurrentEntry] = useState(0)
  const [rootPath, setRootPath] = useState('.')
  const [selectedFilePath, setSelectedFilePath] = useState<string | null>(null)
  const [currentFileName, setCurrentFileName] = useState('editor.hurl')
  const [showExplorer, setShowExplorer] = useState(true)
  const [fileTreeRefreshKey, setFileTreeRefreshKey] = useState(0)
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

  const handleFileSelect = useCallback((path: string, fileContent: string) => {
    setSelectedFilePath(path)
    setCurrentFileName(path.split('/').pop() || path)
    setContent(fileContent)
  }, [])

  const handleCreateFile = useCallback(async (directoryPath: string) => {
    const fileName = window.prompt('New file name', 'new-request.hurl')?.trim()
    if (!fileName) return

    const nextFileName = fileName.endsWith('.hurl') ? fileName : `${fileName}.hurl`
    const baseDir = directoryPath.replace(/\/$/, '')
    const targetPath = `${baseDir}/${nextFileName}`

    const initialContent = 'GET https://jsonplaceholder.typicode.com/todos/1\nHTTP 200\n'

    try {
      const res = await fetch('/api/file', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ path: targetPath, content: initialContent }),
      })

      if (!res.ok) {
        const err = await res.json()
        window.alert(err.error || 'Failed to create file')
        return
      }

      const data = await res.json()
      setSelectedFilePath(data.path)
      setCurrentFileName(nextFileName)
      setContent(initialContent)
      setFileTreeRefreshKey((prev) => prev + 1)
    } catch {
      window.alert('Failed to create file')
    }
  }, [])

  const runRequest = useCallback(async (mode: RunMode, entryIndex?: number) => {
    setIsLoading(true)
    setActiveTab('response')
    try {
      let endpoint = '/api/run-file'
      let body: Record<string, unknown> = { content, env: {} }

      if (mode === 'entry' && entryIndex !== undefined) {
        endpoint = '/api/run-entry'
        body = { content, entry_index: entryIndex, env: {} }
        // Store request info for display
        const entry = entries.find(e => e.index === entryIndex)
        if (entry) {
          setLastRequest({ method: entry.method, url: entry.url })
        }
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
      
      // Extract detailed info from first result
      let firstResult: ExecutionResult | null = null
      if (Array.isArray(data) && data.length > 0) {
        firstResult = data[0]
        setResponseHeaders(data[0].headers || {})
      } else if (data && typeof data === 'object' && 'headers' in data) {
        firstResult = data as ExecutionResult
        setResponseHeaders(data.headers || {})
      } else if (data && typeof data === 'object' && 'results' in data) {
        const results = (data as TestFileResponse).results
        if (results && results.length > 0) {
          firstResult = results[0]
          setResponseHeaders(results[0].headers || {})
        }
      }
      
      // Extract assertions and timing
      if (firstResult) {
        setResponseAssertions(firstResult.assertions || [])
        setResponseTiming(firstResult.timing || null)
      }
    } catch (error) {
      setResponse(`✗ Error: ${error}`)
      setResponseHeaders({})
      setResponseAssertions([])
      setResponseTiming(null)
    } finally {
      setIsLoading(false)
    }
    }, [content, entries])

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
          <div className="flex items-center gap-1 px-2 py-0.5 rounded" style={{ background: 'var(--bg-elevated)' }}>
            <FolderOpen className="w-3 h-3" style={{ color: 'var(--text-muted)' }} />
            <input
              type="text"
              value={rootPath}
              onChange={(e) => setRootPath(e.target.value)}
              className="text-xs bg-transparent border-none outline-none w-32"
              style={{ 
                fontFamily: 'var(--font-mono)',
                color: 'var(--text-secondary)',
              }}
              placeholder="."              
            />
          </div>
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
        {/* File Explorer Sidebar */}
        <div 
          className="w-56 flex flex-col shrink-0"
          style={{ 
            borderRight: '1px solid var(--border-default)',
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
              <FolderOpen className="w-3.5 h-3.5" style={{ color: 'var(--text-muted)' }} />
              <span 
                className="text-xs font-medium"
                style={{ 
                  fontFamily: 'var(--font-mono)',
                  color: 'var(--text-secondary)',
                }}
              >
                EXPLORER
              </span>
            </div>
            <button
              type="button"
              onClick={() => setShowExplorer(!showExplorer)}
              className="p-1 rounded transition-colors hover:bg-[var(--bg-elevated)]"
            >
              <Folder className="w-3.5 h-3.5" style={{ color: 'var(--text-muted)' }} />
            </button>
          </div>
          <div className="flex-1 overflow-y-auto">
            <FileExplorer
              rootPath={rootPath}
              refreshKey={fileTreeRefreshKey}
              onCreateFile={handleCreateFile}
              onFileSelect={handleFileSelect}
              selectedPath={selectedFilePath}
            />
          </div>
        </div>

        {/* Editor Area */}
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
              {currentFileName}
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
          className="w-[450px] flex flex-col min-w-0"
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
            <div className="flex items-center gap-1">
              {/* Tabs */}
              {(['response', 'headers', 'asserts', 'request', 'timing'] as const).map((tab) => (
                <button
                  key={tab}
                  type="button"
                  onClick={() => setActiveTab(tab)}
                  className="px-2 py-1 rounded text-xs capitalize transition-colors"
                  style={{
                    fontFamily: 'var(--font-mono)',
                    color: activeTab === tab ? 'var(--text-primary)' : 'var(--text-muted)',
                    background: activeTab === tab ? 'var(--bg-elevated)' : 'transparent',
                  }}
                >
                  {tab}
                </button>
              ))}
            </div>
            {isLoading && (
              <span className="text-[10px]" style={{ color: 'var(--text-muted)' }}>
                running...
              </span>
            )}
          </div>
          <div className="flex-1 overflow-y-auto p-3">
            {activeTab === 'response' && <ResponseViewer content={response} />}
            
            {activeTab === 'headers' && (
              <div className="text-xs" style={{ fontFamily: 'var(--font-mono)' }}>
                {Object.keys(responseHeaders).length > 0 ? (
                  <div className="space-y-2">
                    {Object.entries(responseHeaders).map(([key, value]) => (
                      <div key={key} className="flex flex-col gap-1">
                        <span style={{ color: 'var(--accent-cyan)' }}>{key}</span>
                        <span style={{ color: 'var(--text-secondary)' }}>{value}</span>
                      </div>
                    ))}
                  </div>
                ) : (
                  <span style={{ color: 'var(--text-muted)' }}>No headers available</span>
                )}
              </div>
            )}
            
            {activeTab === 'asserts' && (
              <div className="text-xs" style={{ fontFamily: 'var(--font-mono)' }}>
                {responseAssertions.length > 0 ? (
                  <div className="space-y-2">
                    {responseAssertions.map((assertion, idx) => (
                      <div key={idx} className="flex items-center gap-2 p-2 rounded" style={{ background: 'var(--bg-elevated)' }}>
                        <span className={assertion.passed ? 'text-green-400' : 'text-red-400'}>
                          {assertion.passed ? '✓' : '✗'}
                        </span>
                        <span style={{ color: 'var(--text-primary)' }}>{assertion.query}</span>
                      </div>
                    ))}
                  </div>
                ) : (
                  <span style={{ color: 'var(--text-muted)' }}>No assertions</span>
                )}
              </div>
            )}
            
            {activeTab === 'request' && (
              <div className="text-xs" style={{ fontFamily: 'var(--font-mono)' }}>
                {lastRequest ? (
                  <div className="space-y-2">
                    <div className="p-2 rounded" style={{ background: 'var(--bg-elevated)' }}>
                      <div className="flex items-center gap-2">
                        <span style={{ color: 'var(--accent-cyan)' }}>{lastRequest.method}</span>
                        <span style={{ color: 'var(--text-secondary)' }}>{lastRequest.url}</span>
                      </div>
                    </div>
                  </div>
                ) : (
                  <span style={{ color: 'var(--text-muted)' }}>No request info</span>
                )}
              </div>
            )}
            
            {activeTab === 'timing' && (
              <div className="text-xs" style={{ fontFamily: 'var(--font-mono)' }}>
                {responseTiming ? (
                  <div className="space-y-2">
                    <div className="p-2 rounded" style={{ background: 'var(--bg-elevated)' }}>
                      <div className="flex justify-between">
                        <span style={{ color: 'var(--text-muted)' }}>Total</span>
                        <span style={{ color: 'var(--accent-cyan)' }}>{responseTiming.duration_ms}ms</span>
                      </div>
                      {responseTiming.connect_time_ms != null && (
                        <div className="flex justify-between mt-1">
                          <span style={{ color: 'var(--text-muted)' }}>Connect</span>
                          <span style={{ color: 'var(--text-secondary)' }}>{responseTiming.connect_time_ms}ms</span>
                        </div>
                      )}
                      {responseTiming.tls_time_ms != null && (
                        <div className="flex justify-between mt-1">
                          <span style={{ color: 'var(--text-muted)' }}>TLS</span>
                          <span style={{ color: 'var(--text-secondary)' }}>{responseTiming.tls_time_ms}ms</span>
                        </div>
                      )}
                      {responseTiming.transfer_time_ms != null && (
                        <div className="flex justify-between mt-1">
                          <span style={{ color: 'var(--text-muted)' }}>Transfer</span>
                          <span style={{ color: 'var(--text-secondary)' }}>{responseTiming.transfer_time_ms}ms</span>
                        </div>
                      )}
                    </div>
                  </div>
                ) : (
                  <span style={{ color: 'var(--text-muted)' }}>No timing info</span>
                )}
              </div>
            )}
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
