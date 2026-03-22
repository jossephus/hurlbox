import { useState, useRef, useCallback, useEffect, type ChangeEvent } from 'react'
import Editor, { type OnMount } from '@monaco-editor/react'
import type * as monaco from 'monaco-editor'
import { TestTube2, FileText, FileCode, Sun, Moon, Loader2, FolderOpen, Folder, Save, Key } from 'lucide-react'
import { ResponseViewer } from './components/ResponseViewer'
import { FileExplorer } from './components/FileExplorer'
import { registerHurlLanguage } from './lib/hurl-lang'
import './App.css'

interface ExecutionResult {
  entry_index: number
  request: {
    method: string
    url: string
    headers: Record<string, string>
    body?: string
  }
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

interface BuildAssertionsResponse {
  content: string
  assertions_added: number
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

function parseEnvInput(input: string): Record<string, string> {
  const env: Record<string, string> = {}
  for (const rawLine of input.split('\n')) {
    const line = rawLine.trim()
    if (!line || line.startsWith('#')) continue
    const withoutExport = line.startsWith('export ') ? line.slice(7).trim() : line
    const separatorIndex = withoutExport.indexOf('=')
    if (separatorIndex === -1) continue
    const key = withoutExport.slice(0, separatorIndex).trim()
    if (!key) continue
    let value = withoutExport.slice(separatorIndex + 1).trim()
    if ((value.startsWith('"') && value.endsWith('"')) || (value.startsWith("'") && value.endsWith("'"))) {
      value = value.slice(1, -1)
    }
    env[key] = value
  }
  return env
}


function App() {
  const [content, setContent] = useState(SAMPLE_HURL)
  const [response, setResponse] = useState<string>('')
  const [responseHeaders, setResponseHeaders] = useState<Record<string, string>>({})
  const [activeTab, setActiveTab] = useState<'response' | 'headers' | 'request'>('response')
  const [isLoading, setIsLoading] = useState(false)
  const [entries, setEntries] = useState<EntryInfo[]>([])
  const [theme, setTheme] = useState<'dark' | 'light'>('dark')
  const [currentEntry, setCurrentEntry] = useState(0)
  const [rootPath, setRootPath] = useState('.')
  const [selectedRelativePath, setSelectedRelativePath] = useState<string | null>(null)
  const [currentFileName, setCurrentFileName] = useState('editor.hurl')
  const [showExplorer, setShowExplorer] = useState(true)
  const [fileTreeRefreshKey, setFileTreeRefreshKey] = useState(0)
  const [envInput, setEnvInput] = useState('')
  const [envFileName, setEnvFileName] = useState<string | null>(null)
  const [serverEnvFileName, setServerEnvFileName] = useState<string | null>(null)
  const [lastRunEntryIndex, setLastRunEntryIndex] = useState<number | null>(null)
  const [lastRunResult, setLastRunResult] = useState<ExecutionResult | null>(null)
  const monacoRef = useRef<typeof monaco | null>(null)
  const editorRef = useRef<monaco.editor.IStandaloneCodeEditor | null>(null)
  const entriesRef = useRef<EntryInfo[]>([])
  const runRequestRef = useRef<((mode: RunMode, entryIndex?: number) => Promise<void>) | null>(null)
  const envFileInputRef = useRef<HTMLInputElement | null>(null)

  useEffect(() => { entriesRef.current = entries }, [entries])

  useEffect(() => {
    const loadServerEnv = async () => {
      try {
        const res = await fetch('/api/env-default')
        if (!res.ok) return
        const data = await res.json()
        if (data.loaded && data.source) {
          setServerEnvFileName(data.source)
        }
      } catch {
        // ignore env metadata failures
      }
    }
    loadServerEnv()
  }, [])

  const toggleTheme = useCallback(() => {
    setTheme(prev => {
      const newTheme = prev === 'dark' ? 'light' : 'dark'
      document.documentElement.classList.toggle('light', newTheme === 'light')
      return newTheme
    })
  }, [])

  const handleFileSelect = useCallback((relativePath: string, fileContent: string, options?: { skipHistory?: boolean }) => {
    setSelectedRelativePath(relativePath)
    setCurrentFileName(relativePath.split('/').pop() || relativePath)
    setContent(fileContent)
    if (!options?.skipHistory) {
      const url = new URL(window.location.href)
      url.searchParams.set('path', relativePath)
      window.history.pushState({ path: relativePath }, '', url.toString())
    }
  }, [])

  useEffect(() => {
    const loadFileFromUrl = async (relativePath: string) => {
      try {
        const res = await fetch(`/api/file?path=${encodeURIComponent(relativePath)}`)
        if (!res.ok) return
        const data = await res.json()
        handleFileSelect(relativePath, data.content, { skipHistory: true })
      } catch {
        // ignore
      }
    }

    const applyFromUrl = () => {
      const url = new URL(window.location.href)
      const relativePath = url.searchParams.get('path')
      if (relativePath) {
        loadFileFromUrl(relativePath)
      }
    }

    applyFromUrl()
    window.addEventListener('popstate', applyFromUrl)
    return () => window.removeEventListener('popstate', applyFromUrl)
  }, [handleFileSelect])

  const handleCreateFile = useCallback(async (directoryRelativePath: string) => {
    const fileName = window.prompt('New file name', 'new-request.hurl')?.trim()
    if (!fileName) return

    const nextFileName = fileName.endsWith('.hurl') ? fileName : `${fileName}.hurl`
    const relativeBase = directoryRelativePath.replace(/\/$/, '')
    const relativePath = `${relativeBase}/${nextFileName}`

    const initialContent = 'GET https://jsonplaceholder.typicode.com/todos/1\nHTTP 200\n'

    try {
      const res = await fetch('/api/file', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ path: relativePath, content: initialContent }),
      })

      if (!res.ok) {
        const err = await res.json()
        window.alert(err.error || 'Failed to create file')
        return
      }

      handleFileSelect(relativePath, initialContent)
      setFileTreeRefreshKey((prev) => prev + 1)
    } catch {
      window.alert('Failed to create file')
    }
  }, [handleFileSelect])

  const handleEnvFileSelected = useCallback(async (event: ChangeEvent<HTMLInputElement>) => {
    const file = event.target.files?.[0]
    if (!file) return
    try {
      const text = await file.text()
      setEnvInput(text)
      setEnvFileName(file.name)
    } catch {
      window.alert('Failed to read env file')
    } finally {
      event.target.value = ''
    }
  }, [])

  const runRequest = useCallback(async (mode: RunMode, entryIndex?: number) => {
    setIsLoading(true)
    setActiveTab('response')
    try {
      const env = parseEnvInput(envInput)
      let endpoint = '/api/run-file'
      let body: Record<string, unknown> = { content, env }

      if (mode === 'entry' && entryIndex !== undefined) {
        endpoint = '/api/run-entry'
        body = { content, entry_index: entryIndex, env }
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
      
      // Store result for request tab and other uses
      if (firstResult) {
        setLastRunResult(firstResult)
      } else {
        setLastRunResult(null)
      }

      if (mode === 'entry' && entryIndex !== undefined) {
        setLastRunEntryIndex(entryIndex)
      } else {
        setLastRunEntryIndex(null)
      }
    } catch (error) {
      setResponse(`✗ Error: ${error}`)
      setResponseHeaders({})
    } finally {
      setIsLoading(false)
    }
    }, [content, envInput])

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

    editor.addCommand(monacoInstance.KeyMod.CtrlCmd | monacoInstance.KeyCode.KeyS, () => {
      handleSaveFile()
    })
  }

  const handleBuildAssertions = useCallback(async () => {
    if (lastRunEntryIndex == null || !lastRunResult) {
      window.alert('Run an entry first to build assertions')
      return
    }

    try {
      const env = parseEnvInput(envInput)
      const res = await fetch('/api/build-assertions', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ content, entry_index: lastRunEntryIndex, env }),
      })

      const data = await res.json()
      if (!res.ok) {
        window.alert(data.error || 'Failed to build assertions')
        return
      }

      const payload = data as BuildAssertionsResponse
      if (payload.content === content || payload.assertions_added === 0) {
        window.alert('No new assertions to add for this entry')
        return
      }

      setContent(payload.content)
      window.alert(`Added ${payload.assertions_added} generated assertion(s) to entry ${lastRunEntryIndex + 1}`)
    } catch {
      window.alert('Failed to build assertions')
    }
  }, [content, envInput, lastRunEntryIndex, lastRunResult])

  const handleSaveFile = useCallback(async () => {
    if (!selectedRelativePath) {
      window.alert('Select a file from explorer first')
      return
    }
    try {
      const res = await fetch('/api/file', {
        method: 'PUT',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ path: selectedRelativePath, content }),
      })
      const data = await res.json()
      if (!res.ok) {
        window.alert(data.error || 'Failed to save file')
        return
      }
      setContent(data.content || content)
    } catch {
      window.alert('Failed to save file')
    }
  }, [content, selectedRelativePath])

  return (
    <div 
      className="h-screen flex flex-col overflow-hidden"
      style={{ background: 'var(--bg-primary)' }}
    >
      <div className="noise-overlay" />

      <header 
        className="app-header shrink-0 px-4 flex items-center justify-between"
        style={{ 
          background: 'var(--bg-secondary)',
          borderBottom: '1px solid var(--border-default)',
        }}
      >
        <div className="app-header-left flex items-center gap-3">
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

        <div className="app-header-actions flex items-center gap-2">
          <button
            type="button"
            onClick={() => setShowExplorer((prev) => !prev)}
            className="p-1.5 rounded transition-colors"
            style={{ color: showExplorer ? 'var(--text-primary)' : 'var(--text-muted)' }}
            aria-label={showExplorer ? 'Hide explorer' : 'Show explorer'}
            title={showExplorer ? 'Hide explorer' : 'Show explorer'}
          >
            <FolderOpen className="w-4 h-4" />
          </button>

          <button
            type="button"
            onClick={() => envFileInputRef.current?.click()}
            className="px-3 py-1.5 text-xs rounded transition-colors flex items-center gap-1.5"
            style={{ 
              color: (envFileName || serverEnvFileName) ? 'var(--text-primary)' : 'var(--text-secondary)',
              background: (envFileName || serverEnvFileName) ? 'var(--bg-elevated)' : 'transparent',
              border: '1px solid var(--border-default)',
            }}
            title={envFileName ? `UI env file: ${envFileName}` : (serverEnvFileName ? `Server env file: ${serverEnvFileName}` : 'Load env file')}
          >
            <Key className="w-3.5 h-3.5" />
            {envFileName ? `Env: ${envFileName}` : (serverEnvFileName ? `Env (server): ${serverEnvFileName}` : 'Load Env')}
          </button>
          <input
            ref={envFileInputRef}
            type="file"
            className="hidden"
            onChange={handleEnvFileSelected}
          />

          <button
            type="button"
            onClick={toggleTheme}
            className="p-1.5 rounded transition-colors"
            style={{ color: 'var(--text-muted)' }}
            aria-label="Toggle theme"
            title="Toggle theme"
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

      <main className="app-main flex-1 flex min-h-0">
        {/* File Explorer Sidebar */}
        <div 
          className={`app-sidebar w-56 flex flex-col shrink-0 ${showExplorer ? 'open' : 'closed'}`}
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
              aria-label={showExplorer ? 'Hide explorer' : 'Show explorer'}
              title={showExplorer ? 'Hide explorer' : 'Show explorer'}
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
              selectedPath={selectedRelativePath}
            />
          </div>
        </div>

        {/* Editor Area */}
        <div className="flex-1 flex flex-col min-w-0">
          <div 
            className="h-8 px-3 flex items-center justify-between shrink-0"
            style={{ 
              background: 'var(--bg-primary)',
              borderBottom: '1px solid var(--border-dim)',
            }}
          >
            <div className="flex items-center min-w-0">
              <FileText className="w-3.5 h-3.5 mr-2 shrink-0" style={{ color: 'var(--text-muted)' }} />
              <span 
                className="text-xs truncate"
                style={{ 
                  fontFamily: 'var(--font-mono)',
                  color: 'var(--text-secondary)',
                }}
              >
                {currentFileName}
              </span>
            </div>
            <button
              type="button"
              onClick={handleSaveFile}
              className="px-2 py-1 text-[10px] rounded transition-colors flex items-center gap-1"
              style={{
                fontFamily: 'var(--font-mono)',
                color: 'var(--text-primary)',
                background: 'var(--bg-elevated)',
                border: '1px solid var(--border-dim)',
              }}
              title="Save file"
            >
              <Save className="w-3 h-3" />
              Save
            </button>
          </div>
          <div className="flex-1 min-h-0">
            <Editor
              height="100%"
              defaultLanguage="hurl"
              theme={theme === 'dark' ? 'vs-dark' : 'vs'}
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
          className="app-response-panel w-[450px] flex flex-col min-w-0 relative"
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
              {(['response', 'headers', 'request'] as const).map((tab) => (
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
            {activeTab === 'response' && <ResponseViewer content={response} theme={theme} />}
            
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
            
            {activeTab === 'request' && (
              <div className="text-xs" style={{ fontFamily: 'var(--font-mono)' }}>
                {lastRunResult?.request ? (
                  <div className="space-y-3">
                    <div className="p-2 rounded" style={{ background: 'var(--bg-elevated)' }}>
                      <div className="flex items-center gap-2">
                        <span style={{ color: 'var(--accent-cyan)' }}>{lastRunResult.request.method}</span>
                        <span style={{ color: 'var(--text-secondary)' }}>{lastRunResult.request.url}</span>
                      </div>
                    </div>
                    {Object.keys(lastRunResult.request.headers).length > 0 && (
                      <div className="p-2 rounded" style={{ background: 'var(--bg-elevated)' }}>
                        <div className="text-[10px] mb-1" style={{ color: 'var(--text-muted)' }}>Request Headers</div>
                        {Object.entries(lastRunResult.request.headers).map(([key, value]) => (
                          <div key={key} className="flex flex-col gap-0.5">
                            <span style={{ color: 'var(--accent-cyan)' }}>{key}</span>
                            <span style={{ color: 'var(--text-secondary)' }}>{value}</span>
                          </div>
                        ))}
                      </div>
                    )}
                    {lastRunResult.request.body && (
                      <div className="p-2 rounded" style={{ background: 'var(--bg-elevated)' }}>
                        <div className="text-[10px] mb-1" style={{ color: 'var(--text-muted)' }}>Request Body</div>
                        <pre className="whitespace-pre-wrap break-all" style={{ color: 'var(--text-secondary)' }}>
                          {lastRunResult.request.body}
                        </pre>
                      </div>
                    )}
                  </div>
                ) : (
                  <span style={{ color: 'var(--text-muted)' }}>No request info</span>
                )}
              </div>
            )}
          </div>
          {response.trim() !== '' && (
            <button
              type="button"
              onClick={handleBuildAssertions}
              className="px-3 py-2 rounded text-xs transition-all duration-200"
              style={{
                position: 'absolute',
                right: '12px',
                bottom: '12px',
                zIndex: 10,
                fontFamily: 'var(--font-mono)',
                color: 'var(--text-primary)',
                background: 'var(--bg-elevated)',
                border: '1px solid var(--border-dim)',
                boxShadow: '0 6px 18px rgba(0,0,0,0.18), 0 0 12px rgba(99, 102, 241, 0.15)',
                opacity: lastRunEntryIndex != null && !!lastRunResult ? 1 : 0.6,
                cursor: lastRunEntryIndex != null && !!lastRunResult ? 'pointer' : 'not-allowed',
              }}
              disabled={lastRunEntryIndex == null || !lastRunResult}
              onMouseEnter={(e) => {
                if (lastRunEntryIndex != null && !!lastRunResult) {
                  e.currentTarget.style.boxShadow = '0 6px 18px rgba(0,0,0,0.25), 0 0 20px rgba(99, 102, 241, 0.3)'
                  e.currentTarget.style.transform = 'translateY(-1px)'
                }
              }}
              onMouseLeave={(e) => {
                e.currentTarget.style.boxShadow = '0 6px 18px rgba(0,0,0,0.18), 0 0 12px rgba(99, 102, 241, 0.15)'
                e.currentTarget.style.transform = 'translateY(0)'
              }}
            >
              Build Assertions
            </button>
          )}
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
        <span>{isLoading ? 'running...' : ''}</span>
        <span>hurlbox</span>
      </footer>
    </div>
  )
}

export default App
